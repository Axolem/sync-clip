//! Axum WebSocket server for `/v0/ws`.

use crate::buffer::{BufferEvent, ChannelBuffer};
use crate::messages::{ClientMessage, ServerMessage};
use crate::{DEFAULT_TTL, MAX_CIPHERTEXT_BYTES};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// Relay runtime configuration.
#[derive(Clone, Debug)]
pub struct RelayConfig {
    pub bind: SocketAddr,
    pub ttl: Duration,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ttl: DEFAULT_TTL,
        }
    }
}

/// Handle to a running relay (for tests and shutdown).
#[derive(Clone)]
pub struct RelayHandle {
    pub buffer: ChannelBuffer,
    pub local_addr: SocketAddr,
    shutdown: Arc<tokio::sync::Notify>,
}

impl RelayHandle {
    pub fn ws_url(&self) -> String {
        format!("ws://{}/v0/ws", self.local_addr)
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}

#[derive(Clone)]
struct AppState {
    buffer: ChannelBuffer,
}

/// Bind and serve the relay. Returns once listening; runs until shutdown notified.
pub async fn start_relay(config: RelayConfig) -> std::io::Result<RelayHandle> {
    let buffer = ChannelBuffer::new(config.ttl);
    let state = AppState {
        buffer: buffer.clone(),
    };
    let app = Router::new()
        .route("/v0/ws", get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind(config.bind).await?;
    let local_addr = listener.local_addr()?;
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_wait = shutdown.clone();

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_wait.notified().await;
            })
            .await
            .ok();
    });

    Ok(RelayHandle {
        buffer,
        local_addr,
        shutdown,
    })
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let mut subscribed: HashSet<String> = HashSet::new();
    let (fanout_tx, mut fanout_rx) = mpsc::unbounded_channel::<BufferEvent>();

    loop {
        tokio::select! {
            evt = fanout_rx.recv() => {
                match evt {
                    Some(event) => {
                        if let Some(msg) = event_to_server(event) {
                            if send_server(&mut sink, &msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
            frame = stream.next() => {
                match frame {
                    Some(Ok(Message::Text(text))) => {
                        let parsed: Result<ClientMessage, _> = serde_json::from_str(&text);
                        match parsed {
                            Ok(ClientMessage::Subscribe { channel_id }) => {
                                if !is_valid_channel_id(&channel_id) {
                                    let _ = send_server(
                                        &mut sink,
                                        &ServerMessage::Error {
                                            code: "bad_request".into(),
                                            message: "invalid channel_id".into(),
                                        },
                                    )
                                    .await;
                                    continue;
                                }
                                let snap = state.buffer.snapshot_event(&channel_id).await;
                                if let Some(msg) = event_to_server(snap) {
                                    if send_server(&mut sink, &msg).await.is_err() {
                                        break;
                                    }
                                }
                                if subscribed.insert(channel_id.clone()) {
                                    let mut rx = state.buffer.subscribe(&channel_id).await;
                                    let tx = fanout_tx.clone();
                                    tokio::spawn(async move {
                                        loop {
                                            match rx.recv().await {
                                                Ok(evt) => {
                                                    if tx.send(evt).is_err() {
                                                        break;
                                                    }
                                                }
                                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                                            }
                                        }
                                    });
                                }
                            }
                            Ok(ClientMessage::Unsubscribe { channel_id }) => {
                                subscribed.remove(&channel_id);
                            }
                            Ok(ClientMessage::Publish {
                                channel_id,
                                ciphertext,
                                nonce,
                                protocol_version,
                            }) => {
                                if let Err(msg) = handle_publish(
                                    &state.buffer,
                                    channel_id,
                                    ciphertext,
                                    nonce,
                                    protocol_version,
                                )
                                .await
                                {
                                    let _ = send_server(&mut sink, &msg).await;
                                }
                            }
                            Err(_) => {
                                let _ = send_server(
                                    &mut sink,
                                    &ServerMessage::Error {
                                        code: "bad_request".into(),
                                        message: "invalid json".into(),
                                    },
                                )
                                .await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        if sink.send(Message::Pong(p)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

async fn handle_publish(
    buffer: &ChannelBuffer,
    channel_id: String,
    ciphertext_b64: String,
    nonce_b64: String,
    protocol_version: u8,
) -> Result<(), ServerMessage> {
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(&ciphertext_b64)
        .map_err(|_| ServerMessage::Error {
            code: "bad_request".into(),
            message: "invalid ciphertext base64".into(),
        })?;
    let nonce = base64::engine::general_purpose::STANDARD
        .decode(&nonce_b64)
        .map_err(|_| ServerMessage::Error {
            code: "bad_request".into(),
            message: "invalid nonce base64".into(),
        })?;
    if ciphertext.len() > MAX_CIPHERTEXT_BYTES {
        return Err(ServerMessage::Error {
            code: "bad_request".into(),
            message: "ciphertext too large".into(),
        });
    }
    buffer
        .publish(channel_id, protocol_version, nonce, ciphertext)
        .await
        .map_err(|e| match e {
            crate::buffer::PublishError::BadChannel => ServerMessage::Error {
                code: "bad_request".into(),
                message: "invalid channel_id".into(),
            },
            crate::buffer::PublishError::TooLarge => ServerMessage::Error {
                code: "bad_request".into(),
                message: "ciphertext too large".into(),
            },
        })?;
    Ok(())
}

fn event_to_server(event: BufferEvent) -> Option<ServerMessage> {
    match event {
        BufferEvent::Empty { channel_id } => Some(ServerMessage::Empty { channel_id }),
        BufferEvent::Envelope(env) => Some(ServerMessage::Envelope {
            channel_id: env.channel_id,
            ciphertext: base64::engine::general_purpose::STANDARD.encode(&env.ciphertext),
            nonce: base64::engine::general_purpose::STANDARD.encode(&env.nonce),
            protocol_version: env.protocol_version,
            published_at: env.published_at_ms,
        }),
    }
}

async fn send_server(
    sink: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<(), ()> {
    let text = serde_json::to_string(msg).map_err(|_| ())?;
    sink.send(Message::Text(text.into())).await.map_err(|_| ())
}

fn is_valid_channel_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
