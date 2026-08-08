//! Public AEAD envelope seal/open seam for Clip plaintext (protocol clip-wire-v0).

use crate::crypto::{derive_aead_key, derive_channel_id};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Shared Sync Group secret: 32 raw bytes (Shell decodes from UX encoding).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinkKey(pub [u8; 32]);

/// Plain-text Clip before encryption (logical model from clip-wire-v0 §1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextClip {
    pub created_at: i64,
    pub id: [u8; 16],
    pub image: Option<ClipImage>,
    pub schema_version: u8,
    pub sender_ephemeral_id: [u8; 16],
    pub text: String,
}

/// Optional image part of a Clip.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Ciphertext envelope fields the relay may store/forward (no plaintext).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedEnvelope {
    pub channel_id: [u8; 16],
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 24],
    pub protocol_version: u8,
}

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("AEAD decrypt failed")]
    Decrypt,
    #[error("CBOR encode/decode failed: {0}")]
    Cbor(String),
    #[error("invalid Clip schema_version: {0}")]
    Schema(u8),
    #[error("ciphertext exceeds 6 MiB soft cap")]
    TooLarge,
}

/// Public seal/open API: encrypts Clip plaintext to a wire envelope and back.
pub struct Envelope;

impl Envelope {
    /// Seal a text Clip into a ciphertext envelope for the Sync Group channel.
    pub fn seal(link_key: &LinkKey, clip: &TextClip) -> Result<SealedEnvelope, EnvelopeError> {
        let channel_id = derive_channel_id(link_key);
        let aead_key = derive_aead_key(link_key);
        let plaintext = encode_clip_cbor(clip)?;

        let mut nonce_bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        let cipher = XChaCha20Poly1305::new_from_slice(&aead_key)
            .expect("AEAD key length is fixed at 32 bytes");
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    aad: &channel_id,
                    msg: &plaintext,
                },
            )
            .map_err(|_| EnvelopeError::Decrypt)?;

        // Soft cap raised to fit ~5 MiB encoded image plaintext + AEAD overhead.
        if ciphertext.len() > 6 * 1024 * 1024 {
            return Err(EnvelopeError::TooLarge);
        }

        Ok(SealedEnvelope {
            channel_id,
            ciphertext,
            nonce: nonce_bytes,
            protocol_version: 1,
        })
    }

    /// Open a ciphertext envelope back to a TextClip using the Link Key.
    pub fn open(link_key: &LinkKey, sealed: &SealedEnvelope) -> Result<TextClip, EnvelopeError> {
        let channel_id = derive_channel_id(link_key);
        if sealed.channel_id != channel_id {
            return Err(EnvelopeError::Decrypt);
        }
        let aead_key = derive_aead_key(link_key);
        let cipher = XChaCha20Poly1305::new_from_slice(&aead_key)
            .expect("AEAD key length is fixed at 32 bytes");
        let nonce = XNonce::from_slice(&sealed.nonce);
        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    aad: &channel_id,
                    msg: &sealed.ciphertext,
                },
            )
            .map_err(|_| EnvelopeError::Decrypt)?;
        decode_clip_cbor(&plaintext)
    }
}

#[derive(Deserialize, Serialize)]
struct CborClip {
    created_at: i64,
    id: serde_bytes::ByteBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<CborImage>,
    schema_version: u8,
    sender_ephemeral_id: serde_bytes::ByteBuf,
    text: String,
}

#[derive(Deserialize, Serialize)]
struct CborImage {
    bytes: serde_bytes::ByteBuf,
    mime: String,
}

fn encode_clip_cbor(clip: &TextClip) -> Result<Vec<u8>, EnvelopeError> {
    if clip.schema_version != 1 {
        return Err(EnvelopeError::Schema(clip.schema_version));
    }
    // Sorted keys via struct field order matching protocol (alphabetical).
    let value = CborClip {
        created_at: clip.created_at,
        id: serde_bytes::ByteBuf::from(clip.id.to_vec()),
        image: clip.image.as_ref().map(|img| CborImage {
            bytes: serde_bytes::ByteBuf::from(img.bytes.clone()),
            mime: img.mime.clone(),
        }),
        schema_version: clip.schema_version,
        sender_ephemeral_id: serde_bytes::ByteBuf::from(clip.sender_ephemeral_id.to_vec()),
        text: clip.text.clone(),
    };
    let mut buf = Vec::new();
    ciborium::into_writer(&value, &mut buf).map_err(|e| EnvelopeError::Cbor(e.to_string()))?;
    Ok(buf)
}

fn decode_clip_cbor(bytes: &[u8]) -> Result<TextClip, EnvelopeError> {
    let value: CborClip =
        ciborium::from_reader(bytes).map_err(|e| EnvelopeError::Cbor(e.to_string()))?;
    if value.schema_version != 1 {
        return Err(EnvelopeError::Schema(value.schema_version));
    }
    let id = bytes_to_array_16(&value.id)?;
    let sender_ephemeral_id = bytes_to_array_16(&value.sender_ephemeral_id)?;
    Ok(TextClip {
        created_at: value.created_at,
        id,
        image: value.image.map(|img| ClipImage {
            bytes: img.bytes.into_vec(),
            mime: img.mime,
        }),
        schema_version: value.schema_version,
        sender_ephemeral_id,
        text: value.text,
    })
}

fn bytes_to_array_16(buf: &serde_bytes::ByteBuf) -> Result<[u8; 16], EnvelopeError> {
    let slice = buf.as_slice();
    if slice.len() != 16 {
        return Err(EnvelopeError::Cbor(format!(
            "expected 16 bytes, got {}",
            slice.len()
        )));
    }
    let mut out = [0u8; 16];
    out.copy_from_slice(slice);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_channel_id;

    fn sample_link_key() -> LinkKey {
        LinkKey([0x11; 32])
    }

    fn sample_clip() -> TextClip {
        TextClip {
            created_at: 1_710_000_000_000,
            id: [0xAB; 16],
            image: None,
            schema_version: 1,
            sender_ephemeral_id: [0xCD; 16],
            text: "hello sync-clip".to_string(),
        }
    }

    #[test]
    fn seal_open_roundtrips_text_clip() {
        let link_key = sample_link_key();
        let clip = sample_clip();

        let sealed = Envelope::seal(&link_key, &clip).expect("seal");
        assert_eq!(sealed.protocol_version, 1);
        assert_eq!(sealed.nonce.len(), 24);
        assert!(!sealed.ciphertext.is_empty());
        // Ciphertext must not contain plaintext.
        let ct_str = String::from_utf8_lossy(&sealed.ciphertext);
        assert!(!ct_str.contains("hello sync-clip"));

        let opened = Envelope::open(&link_key, &sealed).expect("open");
        assert_eq!(opened, clip);
    }

    #[test]
    fn channel_id_is_stable_hex_for_link_key() {
        let link_key = sample_link_key();
        let a = derive_channel_id(&link_key);
        let b = derive_channel_id(&link_key);
        assert_eq!(a, b);
        let hex = hex::encode(a);
        assert_eq!(hex.len(), 32);
        assert_eq!(hex, hex.to_lowercase());
    }

    #[test]
    fn wrong_link_key_cannot_open() {
        let sealed = Envelope::seal(&sample_link_key(), &sample_clip()).expect("seal");
        let other = LinkKey([0x22; 32]);
        assert!(Envelope::open(&other, &sealed).is_err());
    }

    #[test]
    fn seal_open_roundtrips_image_part() {
        let link_key = sample_link_key();
        let mut clip = sample_clip();
        clip.image = Some(ClipImage {
            bytes: b"fake-png-bytes".to_vec(),
            mime: "image/png".into(),
        });
        let sealed = Envelope::seal(&link_key, &clip).expect("seal");
        let opened = Envelope::open(&link_key, &sealed).expect("open");
        assert_eq!(opened, clip);
    }

    #[test]
    fn clip_cbor_plaintext_never_contains_local_nickname_key() {
        let clip = sample_clip();
        let bytes = encode_clip_cbor(&clip).expect("encode");
        let value: ciborium::value::Value =
            ciborium::from_reader(bytes.as_slice()).expect("cbor map");
        let ciborium::value::Value::Map(entries) = value else {
            panic!("expected CBOR map");
        };
        let keys: Vec<String> = entries
            .iter()
            .filter_map(|(k, _)| match k {
                ciborium::value::Value::Text(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !keys.iter().any(|k| k == "nickname" || k == "local_nickname"),
            "Clip plaintext must never carry Local Nickname; keys={keys:?}"
        );
        assert!(keys.contains(&"text".to_string()));
        assert!(keys.contains(&"schema_version".to_string()));
    }
}
