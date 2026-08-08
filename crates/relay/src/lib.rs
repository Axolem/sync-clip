//! Encrypted relay: WebSocket publish/subscribe of opaque Clip envelopes.
//!
//! Never decrypts. Stores at most one envelope per channel with a configurable TTL.

mod buffer;
mod messages;
mod server;

pub use buffer::{ChannelBuffer, StoredEnvelope};
pub use server::{RelayConfig, RelayHandle, start_relay};

/// Soft ciphertext size cap: 6 MiB (fits ~5 MiB encoded Clip image + AEAD overhead).
/// clip-wire-v0 suggested 2 MiB; raised so in-cap images can traverse the relay.
pub const MAX_CIPHERTEXT_BYTES: usize = 6 * 1024 * 1024;

/// Default buffer TTL (~15 minutes).
pub const DEFAULT_TTL: std::time::Duration = std::time::Duration::from_secs(15 * 60);
