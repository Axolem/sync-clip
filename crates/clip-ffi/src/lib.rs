//! Blocking Session facade over the Clip Engine for native Shell FFI (UniFFI).

uniffi::setup_scaffolding!();

use clip_engine::{
    boot_should_force_paused, capture_missing_should_persist_paused, ensure_rustls_crypto_provider,
    may_auto_start, may_enter_armed, should_keep_lifetime, AppliedClip, Device, DeviceError,
    LinkKey, LifetimeSnapshot, MAX_IMAGE_BYTES,
};
use data_encoding::BASE32_NOPAD;
use rand::RngCore;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::runtime::Runtime;

/// Default hosted relay WebSocket URL for Shells (Caddy → Sync Clip relay).
pub const DEFAULT_RELAY_WS_URL: &str = "wss://clip.dotenv.co.za/v0/ws";

/// Applied Clip surfaced to Shells over FFI.
#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct AppliedClipFfi {
    pub created_at: i64,
    pub id_hex: String,
    pub image_bytes: Option<Vec<u8>>,
    pub image_mime: Option<String>,
    pub text: String,
}

#[derive(Debug, Error, uniffi::Error)]
pub enum SessionError {
    #[error("invalid Link Key length (need 32 bytes)")]
    InvalidLinkKey,
    #[error("invalid ephemeral id length (need 16 bytes)")]
    InvalidEphemeralId,
    #[error("invalid Link Key base32: {0}")]
    InvalidBase32(String),
    #[error("failed to join Sync Group: {0}")]
    JoinFailed(String),
    #[error("device is Paused")]
    Paused,
    #[error("session closed")]
    Closed,
    #[error("{0}")]
    Other(String),
}

impl From<DeviceError> for SessionError {
    fn from(value: DeviceError) -> Self {
        match value {
            DeviceError::Paused => SessionError::Paused,
            DeviceError::Closed => SessionError::Closed,
            other => SessionError::Other(other.to_string()),
        }
    }
}

/// Generate a fresh 32-byte Link Key.
#[uniffi::export]
pub fn generate_link_key() -> Vec<u8> {
    let mut bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Encode Link Key bytes as unpadded base32 (Shell UX).
#[uniffi::export]
pub fn link_key_to_base32(key: Vec<u8>) -> String {
    BASE32_NOPAD.encode(&key)
}

/// Decode an unpadded base32 Link Key into raw bytes.
#[uniffi::export]
pub fn link_key_from_base32(encoded: String) -> Result<Vec<u8>, SessionError> {
    let cleaned = encoded
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    BASE32_NOPAD
        .decode(cleaned.as_bytes())
        .map_err(|e| SessionError::InvalidBase32(e.to_string()))
}

/// Generate a 16-byte sender ephemeral id (echo suppression; not identity).
#[uniffi::export]
pub fn generate_ephemeral_id() -> Vec<u8> {
    let mut bytes = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Default relay WebSocket URL helper for Shell settings.
#[uniffi::export]
pub fn default_relay_ws_url() -> String {
    DEFAULT_RELAY_WS_URL.to_string()
}

/// Encoded image soft cap (~5 MiB) exposed for Shell capture policy.
#[uniffi::export]
pub fn max_image_bytes() -> u64 {
    MAX_IMAGE_BYTES as u64
}

/// Shell Lifetime inputs for resume-on-boot / Arm / capture gates (ADR-0006).
#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct LifetimeSnapshotFfi {
    pub durable_armed: bool,
    pub elevated_capture_granted: bool,
    pub has_link_key: bool,
    pub quit_opted_out: bool,
    pub requires_elevated_capture: bool,
}

impl From<LifetimeSnapshotFfi> for LifetimeSnapshot {
    fn from(value: LifetimeSnapshotFfi) -> Self {
        Self {
            durable_armed: value.durable_armed,
            elevated_capture_granted: value.elevated_capture_granted,
            has_link_key: value.has_link_key,
            quit_opted_out: value.quit_opted_out,
            requires_elevated_capture: value.requires_elevated_capture,
        }
    }
}

#[uniffi::export]
pub fn lifetime_may_auto_start(snapshot: LifetimeSnapshotFfi) -> bool {
    may_auto_start(&snapshot.into())
}

#[uniffi::export]
pub fn lifetime_may_enter_armed(snapshot: LifetimeSnapshotFfi) -> bool {
    may_enter_armed(&snapshot.into())
}

#[uniffi::export]
pub fn lifetime_boot_should_force_paused(snapshot: LifetimeSnapshotFfi) -> bool {
    boot_should_force_paused(&snapshot.into())
}

#[uniffi::export]
pub fn lifetime_capture_missing_should_persist_paused(
    requires_elevated_capture: bool,
    elevated_capture_granted: bool,
) -> bool {
    capture_missing_should_persist_paused(requires_elevated_capture, elevated_capture_granted)
}

#[uniffi::export]
pub fn lifetime_should_keep_lifetime(has_link_key: bool) -> bool {
    should_keep_lifetime(has_link_key)
}

fn parse_link_key(bytes: &[u8]) -> Result<LinkKey, SessionError> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SessionError::InvalidLinkKey)?;
    Ok(LinkKey(arr))
}

fn parse_ephemeral(bytes: &[u8]) -> Result<[u8; 16], SessionError> {
    bytes
        .try_into()
        .map_err(|_| SessionError::InvalidEphemeralId)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn applied_to_ffi(applied: AppliedClip) -> AppliedClipFfi {
    let (image_bytes, image_mime) = match applied.image {
        Some(img) => (Some(img.bytes), Some(img.mime)),
        None => (None, None),
    };
    AppliedClipFfi {
        created_at: applied.created_at,
        id_hex: hex::encode(applied.id),
        image_bytes,
        image_mime,
        text: applied.text,
    }
}

/// Sync/blocking Session wrapping a Clip Engine Device on an owned Tokio runtime.
#[derive(uniffi::Object)]
pub struct Session {
    device: Mutex<Device>,
    runtime: Runtime,
}

#[uniffi::export]
impl Session {
    /// Join a Sync Group (blocking). Starts Armed.
    #[uniffi::constructor]
    pub fn new(
        link_key_bytes: Vec<u8>,
        relay_ws_url: String,
        ephemeral_id_bytes: Vec<u8>,
    ) -> Result<std::sync::Arc<Self>, SessionError> {
        // Before any rustls ClientConfig::builder() path can run in this staticlib.
        ensure_rustls_crypto_provider();
        let link_key = parse_link_key(&link_key_bytes)?;
        let ephemeral = parse_ephemeral(&ephemeral_id_bytes)?;
        let runtime = Runtime::new().map_err(|e| SessionError::Other(e.to_string()))?;
        let device = runtime
            .block_on(Device::join(link_key, relay_ws_url, ephemeral))
            .map_err(|e| SessionError::JoinFailed(e.to_string()))?;
        Ok(std::sync::Arc::new(Self {
            device: Mutex::new(device),
            runtime,
        }))
    }

    pub fn set_armed(&self, armed: bool) {
        let mut device = self.device.lock().expect("session lock");
        let _ = self.runtime.block_on(device.set_armed(armed));
    }

    pub fn is_armed(&self) -> bool {
        self.device.lock().expect("session lock").is_armed()
    }

    /// Sync Idle: joined Device retrying after relay drop (not Paused).
    pub fn is_sync_idle(&self) -> bool {
        self.device.lock().expect("session lock").is_sync_idle()
    }

    /// Publish plain text while Armed (created_at = now millis).
    pub fn publish_text(&self, text: String) -> Result<(), SessionError> {
        let mut device = self.device.lock().expect("session lock");
        self.runtime
            .block_on(device.publish_text(&text, now_millis()))
            .map(|_| ())
            .map_err(SessionError::from)
    }

    /// Publish text plus optional image. Oversized images are omitted by the Clip Engine;
    /// text still syncs. Local Nickname is intentionally not a parameter (Shell-only).
    pub fn publish_text_and_image(
        &self,
        text: String,
        image_bytes: Vec<u8>,
        image_mime: String,
    ) -> Result<(), SessionError> {
        let mut device = self.device.lock().expect("session lock");
        self.runtime
            .block_on(device.publish(
                &text,
                Some((image_bytes, image_mime)),
                now_millis(),
            ))
            .map(|_| ())
            .map_err(SessionError::from)
    }

    /// Non-blocking poll for the next applied remote Clip.
    pub fn poll_applied(&self) -> Option<AppliedClipFfi> {
        let mut device = self.device.lock().expect("session lock");
        device.try_applied_clip().map(applied_to_ffi)
    }
}
