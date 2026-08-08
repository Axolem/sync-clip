//! WebSocket JSON message shapes (clip-wire-v0 §3.2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Publish {
        channel_id: String,
        ciphertext: String,
        nonce: String,
        protocol_version: u8,
    },
    Subscribe {
        channel_id: String,
    },
    Unsubscribe {
        channel_id: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Empty {
        channel_id: String,
    },
    Envelope {
        channel_id: String,
        ciphertext: String,
        nonce: String,
        protocol_version: u8,
        published_at: i64,
    },
    Error {
        code: String,
        message: String,
    },
}
