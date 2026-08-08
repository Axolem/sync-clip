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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// How often the relay logs the live WebSocket connection count.
const CONNECTION_STATS_INTERVAL: Duration = Duration::from_secs(30);

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
    abort: tokio::task::AbortHandle,
    pub buffer: ChannelBuffer,
    connections: Arc<AtomicUsize>,
    pub local_addr: SocketAddr,
    shutdown: Arc<tokio::sync::Notify>,
}

impl RelayHandle {
    pub fn connected_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}/v0/ws", self.local_addr)
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
        // Abort the serve task so open WebSockets drop immediately (Sync Idle tests / restarts).
        self.abort.abort();
    }
}

#[derive(Clone)]
struct AppState {
    buffer: ChannelBuffer,
    connections: Arc<AtomicUsize>,
    shutdown: Arc<tokio::sync::Notify>,
}

/// Decrements the live connection counter when a WebSocket task ends.
struct ConnectionGuard {
    connections: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let previous = self.connections.fetch_sub(1, Ordering::SeqCst);
        tracing::info!(
            connected = previous.saturating_sub(1),
            "websocket disconnected"
        );
    }
}

/// Bind and serve the relay. Returns once listening; runs until shutdown notified.
pub async fn start_relay(config: RelayConfig) -> std::io::Result<RelayHandle> {
    let buffer = ChannelBuffer::new(config.ttl);
    let connections = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let state = AppState {
        buffer: buffer.clone(),
        connections: connections.clone(),
        shutdown: shutdown.clone(),
    };
    let app = Router::new()
        .route("/v0/ws", get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind(config.bind).await?;
    let local_addr = listener.local_addr()?;
    let shutdown_wait = shutdown.clone();
    let shutdown_stats = shutdown.clone();
    let connections_stats = connections.clone();

    let serve = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_wait.notified().await;
            })
            .await
            .ok();
    });
    let abort = serve.abort_handle();

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(CONNECTION_STATS_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the immediate first tick so the first log is after one full interval.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = shutdown_stats.notified() => break,
                _ = ticker.tick() => {
                    tracing::info!(
                        connected = connections_stats.load(Ordering::SeqCst),
                        "relay connections"
                    );
                }
            }
        }
    });

    Ok(RelayHandle {
        abort,
        buffer,
        connections,
        local_addr,
        shutdown,
    })
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let connected = state.connections.fetch_add(1, Ordering::SeqCst) + 1;
    tracing::info!(connected, "websocket connected");
    let _guard = ConnectionGuard {
        connections: state.connections.clone(),
    };

    let (mut sink, mut stream) = socket.split();
    let mut subscribed: HashSet<String> = HashSet::new();
    let (fanout_tx, mut fanout_rx) = mpsc::unbounded_channel::<BufferEvent>();
    let shutdown = state.shutdown.clone();

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            evt = fanout_rx.recv() => {
                match evt {
                    Some(event) => {
                        if let Some(msg) = event_to_server(event) {
                            log_outbound(&msg);
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
                                tracing::info!(
                                    channel_id = %channel_id,
                                    direction = "in",
                                    msg_type = "subscribe",
                                    "relay message"
                                );
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
                                    log_outbound(&msg);
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
                                tracing::info!(
                                    channel_id = %channel_id,
                                    direction = "in",
                                    msg_type = "unsubscribe",
                                    "relay message"
                                );
                                subscribed.remove(&channel_id);
                            }
                            Ok(ClientMessage::Publish {
                                channel_id,
                                ciphertext,
                                nonce,
                                protocol_version,
                            }) => {
                                tracing::info!(
                                    channel_id = %channel_id,
                                    ciphertext_b64_len = ciphertext.len(),
                                    direction = "in",
                                    msg_type = "publish",
                                    nonce_b64_len = nonce.len(),
                                    protocol_version,
                                    "relay message"
                                );
                                if let Err(msg) = handle_publish(
                                    &state.buffer,
                                    channel_id,
                                    ciphertext,
                                    nonce,
                                    protocol_version,
                                )
                                .await
                                {
                                    log_outbound(&msg);
                                    let _ = send_server(&mut sink, &msg).await;
                                }
                            }
                            Err(_) => {
                                tracing::warn!(
                                    direction = "in",
                                    msg_type = "invalid_json",
                                    "relay message"
                                );
                                let err = ServerMessage::Error {
                                    code: "bad_request".into(),
                                    message: "invalid json".into(),
                                };
                                log_outbound(&err);
                                let _ = send_server(&mut sink, &err).await;
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

fn log_outbound(msg: &ServerMessage) {
    match msg {
        ServerMessage::Empty { channel_id } => {
            tracing::info!(
                channel_id = %channel_id,
                direction = "out",
                msg_type = "empty",
                "relay message"
            );
        }
        ServerMessage::Envelope {
            channel_id,
            ciphertext,
            nonce,
            protocol_version,
            published_at,
        } => {
            tracing::info!(
                channel_id = %channel_id,
                ciphertext_b64_len = ciphertext.len(),
                direction = "out",
                msg_type = "envelope",
                nonce_b64_len = nonce.len(),
                protocol_version,
                published_at,
                "relay message"
            );
        }
        ServerMessage::Error { code, message } => {
            tracing::info!(
                code = %code,
                direction = "out",
                message = %message,
                msg_type = "error",
                "relay message"
            );
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
