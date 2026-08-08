//! Clip Engine — platform-agnostic core for Sync Clip.
//!
//! Owns the Clip model, end-to-end encryption, sync protocol, Armed/Paused
//! rules, and echo suppression. Native Shells link this crate over FFI.

mod crypto;
mod device;
mod envelope;
mod lifetime;

pub use crypto::{channel_id_hex, derive_aead_key, derive_channel_id};
pub use device::{
    ensure_rustls_crypto_provider, AppliedClip, ClipId, Device, DeviceError, MAX_IMAGE_BYTES,
};
pub use envelope::{ClipImage, Envelope, EnvelopeError, LinkKey, SealedEnvelope, TextClip};
pub use lifetime::{
    boot_should_force_paused, capture_missing_should_persist_paused, may_auto_start,
    may_enter_armed, should_keep_lifetime, LifetimeSnapshot,
};

/// Semantic version of the Clip Engine library crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Lightweight readiness probe for FFI / Shell wiring checks.
pub fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonempty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn ping_returns_pong() {
        assert_eq!(ping(), "pong");
    }
}
