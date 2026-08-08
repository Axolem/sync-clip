//! Link Key → channel id and AEAD key via HKDF-SHA256 (clip-wire-v0 §2.2).

use crate::envelope::LinkKey;
use hkdf::Hkdf;
use sha2::Sha256;

const INFO_CHANNEL_ID: &[u8] = b"sync-clip/v0/channel-id";
const INFO_AEAD_KEY: &[u8] = b"sync-clip/v0/clip-aead-key";

/// Derive the public 16-byte Sync Group channel id from a Link Key.
pub fn derive_channel_id(link_key: &LinkKey) -> [u8; 16] {
    let mut out = [0u8; 16];
    hkdf_expand(link_key.0.as_slice(), INFO_CHANNEL_ID, &mut out);
    out
}

/// Lowercase hex encoding of the channel id (32 hex chars) for the wire.
pub fn channel_id_hex(link_key: &LinkKey) -> String {
    hex::encode(derive_channel_id(link_key))
}

/// Derive the private 32-byte Clip AEAD key from a Link Key.
pub fn derive_aead_key(link_key: &LinkKey) -> [u8; 32] {
    let mut out = [0u8; 32];
    hkdf_expand(link_key.0.as_slice(), INFO_AEAD_KEY, &mut out);
    out
}

fn hkdf_expand(ikm: &[u8], info: &[u8], out: &mut [u8]) {
    // Salt empty: Link Key entropy is the salt (protocol §2.2).
    let hk = Hkdf::<Sha256>::new(Some(&[]), ikm);
    hk.expand(info, out)
        .expect("HKDF expand length fits OKM limits");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_link_key_produces_stable_channel_and_aead() {
        let key = LinkKey([0x42; 32]);
        let channel = derive_channel_id(&key);
        let aead = derive_aead_key(&key);
        assert_ne!(channel.as_slice(), &aead[..16]);
        assert_eq!(derive_channel_id(&key), channel);
        assert_eq!(derive_aead_key(&key), aead);
        assert_eq!(channel_id_hex(&key), hex::encode(channel));
    }
}
