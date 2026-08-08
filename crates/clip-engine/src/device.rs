//! Device API: join Sync Group, Armed/Paused, publish/apply Clips.

use crate::crypto::{channel_id_hex, derive_channel_id};
use crate::envelope::{Envelope, LinkKey, SealedEnvelope, TextClip};
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

/// Opaque Clip id (16 bytes).
pub type ClipId = [u8; 16];

/// A remote Clip that passed decrypt, Armed, echo, and LWW checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedClip {
    pub created_at: i64,
    pub id: ClipId,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("device is Paused")]
    Paused,
    #[error("websocket error: {0}")]
    WebSocket(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("channel closed")]
    Closed,
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// One Device joined to a Sync Group via Link Key and relay URL.
pub struct Device {
    applied_rx: mpsc::UnboundedReceiver<AppliedClip>,
    armed: bool,
    cmd_tx: mpsc::UnboundedSender<DeviceCommand>,
    sender_ephemeral_id: [u8; 16],
}

enum DeviceCommand {
    Publish {
        clip: TextClip,
        reply: tokio::sync::oneshot::Sender<Result<ClipId, DeviceError>>,
    },
    SetArmed {
        armed: bool,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    Shutdown,
}

impl Device {
    /// Join a Sync Group: derive channel, connect to relay, subscribe.
    /// Starts Armed by default.
    pub async fn join(
        link_key: LinkKey,
        relay_url: impl Into<String>,
        sender_ephemeral_id: [u8; 16],
    ) -> Result<Self, DeviceError> {
        let relay_url = relay_url.into();
        let channel_hex = channel_id_hex(&link_key);
        let (ws, _) = connect_async(&relay_url)
            .await
            .map_err(|e| DeviceError::WebSocket(e.to_string()))?;

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (applied_tx, applied_rx) = mpsc::unbounded_channel();

        let runtime_key = link_key;
        let runtime_sender = sender_ephemeral_id;
        tokio::spawn(async move {
            device_runtime(
                ws,
                channel_hex,
                runtime_key,
                runtime_sender,
                cmd_rx,
                applied_tx,
            )
            .await;
        });

        Ok(Self {
            applied_rx,
            armed: true,
            cmd_tx,
            sender_ephemeral_id,
        })
    }

    /// Set Armed/Paused. Waits until the Device runtime acknowledges.
    pub async fn set_armed(&mut self, armed: bool) -> Result<(), DeviceError> {
        self.armed = armed;
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(DeviceCommand::SetArmed {
                armed,
                reply: reply_tx,
            })
            .map_err(|_| DeviceError::Closed)?;
        reply_rx.await.map_err(|_| DeviceError::Closed)
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Publish a plain-text Clip while Armed. Returns the Clip id.
    pub async fn publish_text(
        &mut self,
        text: &str,
        created_at_ms: i64,
    ) -> Result<ClipId, DeviceError> {
        let id = Uuid::new_v4().into_bytes();
        self.publish_text_with_id(text, created_at_ms, id).await
    }

    /// Wait for the next remote Clip that was applied (after LWW / echo / Armed).
    pub async fn next_applied_clip(&mut self) -> Result<AppliedClip, DeviceError> {
        self.applied_rx.recv().await.ok_or(DeviceError::Closed)
    }

    /// Non-blocking poll for an applied remote Clip (FFI Session facade).
    pub fn try_applied_clip(&mut self) -> Option<AppliedClip> {
        self.applied_rx.try_recv().ok()
    }

    /// Publish a Clip with an explicit id (tests / echo simulation).
    pub async fn publish_text_with_id(
        &mut self,
        text: &str,
        created_at_ms: i64,
        id: ClipId,
    ) -> Result<ClipId, DeviceError> {
        if !self.armed {
            return Err(DeviceError::Paused);
        }
        let clip = TextClip {
            created_at: created_at_ms,
            id,
            image: None,
            schema_version: 1,
            sender_ephemeral_id: self.sender_ephemeral_id,
            text: text.to_string(),
        };
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(DeviceCommand::Publish {
                clip,
                reply: reply_tx,
            })
            .map_err(|_| DeviceError::Closed)?;
        reply_rx.await.map_err(|_| DeviceError::Closed)?
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(DeviceCommand::Shutdown);
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Publish {
        channel_id: String,
        ciphertext: String,
        nonce: String,
        protocol_version: u8,
    },
    Subscribe {
        channel_id: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg {
    Empty {
        #[allow(dead_code)]
        channel_id: String,
    },
    Envelope {
        #[allow(dead_code)]
        channel_id: String,
        ciphertext: String,
        nonce: String,
        protocol_version: u8,
        #[allow(dead_code)]
        published_at: i64,
    },
    Error {
        code: String,
        message: String,
    },
}

struct RuntimeState {
    armed: bool,
    last_applied: Option<TextClip>,
    last_remote_applied: Option<TextClip>,
    link_key: LinkKey,
    sender_ephemeral_id: [u8; 16],
}

async fn device_runtime(
    mut ws: Ws,
    channel_hex: String,
    link_key: LinkKey,
    sender_ephemeral_id: [u8; 16],
    mut cmd_rx: mpsc::UnboundedReceiver<DeviceCommand>,
    applied_tx: mpsc::UnboundedSender<AppliedClip>,
) {
    let subscribe = ClientMsg::Subscribe {
        channel_id: channel_hex.clone(),
    };
    if send_json(&mut ws, &subscribe).await.is_err() {
        return;
    }

    let mut state = RuntimeState {
        armed: true,
        last_applied: None,
        last_remote_applied: None,
        link_key,
        sender_ephemeral_id,
    };

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    None | Some(DeviceCommand::Shutdown) => break,
                    Some(DeviceCommand::SetArmed { armed, reply }) => {
                        state.armed = armed;
                        let _ = reply.send(());
                    }
                    Some(DeviceCommand::Publish { clip, reply }) => {
                        let result = handle_publish(&mut ws, &mut state, &channel_hex, clip).await;
                        let _ = reply.send(result);
                    }
                }
            }
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_server_text(&text, &mut state, &applied_tx);
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = ws.send(Message::Pong(p)).await;
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
    ws: &mut Ws,
    state: &mut RuntimeState,
    channel_hex: &str,
    clip: TextClip,
) -> Result<ClipId, DeviceError> {
    if !state.armed {
        return Err(DeviceError::Paused);
    }
    // Echo suppression: do not re-broadcast last remotely applied Clip.
    if let Some(ref remote) = state.last_remote_applied {
        if remote.id == clip.id
            || (remote.text == clip.text && remote.created_at == clip.created_at)
        {
            return Ok(clip.id);
        }
    }
    let sealed =
        Envelope::seal(&state.link_key, &clip).map_err(|e| DeviceError::Crypto(e.to_string()))?;
    let msg = ClientMsg::Publish {
        channel_id: channel_hex.to_string(),
        ciphertext: base64::engine::general_purpose::STANDARD.encode(&sealed.ciphertext),
        nonce: base64::engine::general_purpose::STANDARD.encode(sealed.nonce),
        protocol_version: sealed.protocol_version,
    };
    send_json(ws, &msg)
        .await
        .map_err(DeviceError::WebSocket)?;
    state.last_applied = Some(clip.clone());
    Ok(clip.id)
}

fn handle_server_text(
    text: &str,
    state: &mut RuntimeState,
    applied_tx: &mpsc::UnboundedSender<AppliedClip>,
) {
    let Ok(msg) = serde_json::from_str::<ServerMsg>(text) else {
        return;
    };
    match msg {
        ServerMsg::Envelope {
            ciphertext,
            nonce,
            protocol_version,
            ..
        } => {
            if !state.armed {
                return;
            }
            let Ok(ct) = base64::engine::general_purpose::STANDARD.decode(&ciphertext) else {
                return;
            };
            let Ok(nonce_bytes) = base64::engine::general_purpose::STANDARD.decode(&nonce) else {
                return;
            };
            if nonce_bytes.len() != 24 {
                return;
            }
            let mut nonce_arr = [0u8; 24];
            nonce_arr.copy_from_slice(&nonce_bytes);
            let sealed = SealedEnvelope {
                channel_id: derive_channel_id(&state.link_key),
                ciphertext: ct,
                nonce: nonce_arr,
                protocol_version,
            };
            let Ok(clip) = Envelope::open(&state.link_key, &sealed) else {
                return;
            };
            // Ignore own fanout echo.
            if clip.sender_ephemeral_id == state.sender_ephemeral_id {
                return;
            }
            if !wins_lww(state.last_applied.as_ref(), &clip) {
                return;
            }
            let applied = AppliedClip {
                created_at: clip.created_at,
                id: clip.id,
                text: clip.text.clone(),
            };
            state.last_remote_applied = Some(clip.clone());
            state.last_applied = Some(clip);
            let _ = applied_tx.send(applied);
        }
        ServerMsg::Empty { .. } => {}
        ServerMsg::Error { code, message } => {
            let _ = (code, message);
        }
    }
}

fn wins_lww(current: Option<&TextClip>, remote: &TextClip) -> bool {
    match current {
        None => true,
        Some(cur) => {
            if remote.created_at != cur.created_at {
                remote.created_at > cur.created_at
            } else {
                remote.id > cur.id
            }
        }
    }
}

async fn send_json(ws: &mut Ws, msg: &impl Serialize) -> Result<(), String> {
    let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}
