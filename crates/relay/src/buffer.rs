//! Latest-only per-channel envelope buffer with TTL.

use crate::MAX_CIPHERTEXT_BYTES;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

/// Opaque envelope stored by the relay (ciphertext fields only).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredEnvelope {
    pub channel_id: String,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub protocol_version: u8,
    pub published_at_ms: i64,
}

#[derive(Clone)]
struct ChannelState {
    envelope: StoredEnvelope,
    expires_at: Instant,
}

/// In-memory latest-only buffers keyed by channel_id (lowercase hex).
#[derive(Clone)]
pub struct ChannelBuffer {
    inner: Arc<RwLock<HashMap<String, ChannelState>>>,
    subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<BufferEvent>>>>,
    ttl: Duration,
    clock: Clock,
}

#[derive(Clone)]
enum Clock {
    System,
    #[cfg(test)]
    Manual(Arc<RwLock<Instant>>),
}

/// Events fanned out to channel subscribers.
#[derive(Clone, Debug)]
pub enum BufferEvent {
    Empty { channel_id: String },
    Envelope(StoredEnvelope),
}

impl ChannelBuffer {
    pub fn new(ttl: Duration) -> Self {
        Self {
            clock: Clock::System,
            inner: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    #[cfg(test)]
    pub fn with_manual_clock(ttl: Duration, start: Instant) -> (Self, Arc<RwLock<Instant>>) {
        let clock_handle = Arc::new(RwLock::new(start));
        let buf = Self {
            clock: Clock::Manual(clock_handle.clone()),
            inner: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        };
        (buf, clock_handle)
    }

    async fn now(&self) -> Instant {
        match &self.clock {
            Clock::System => Instant::now(),
            #[cfg(test)]
            Clock::Manual(h) => *h.read().await,
        }
    }

    fn published_at_ms_now() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Replace the channel buffer with a new envelope; fan out to subscribers.
    pub async fn publish(
        &self,
        channel_id: String,
        protocol_version: u8,
        nonce: Vec<u8>,
        ciphertext: Vec<u8>,
    ) -> Result<StoredEnvelope, PublishError> {
        if ciphertext.len() > MAX_CIPHERTEXT_BYTES {
            return Err(PublishError::TooLarge);
        }
        if !is_valid_channel_id(&channel_id) {
            return Err(PublishError::BadChannel);
        }
        let now = self.now().await;
        let stored = StoredEnvelope {
            channel_id: channel_id.clone(),
            ciphertext,
            nonce,
            protocol_version,
            published_at_ms: Self::published_at_ms_now(),
        };
        {
            let mut map = self.inner.write().await;
            map.insert(
                channel_id.clone(),
                ChannelState {
                    envelope: stored.clone(),
                    expires_at: now + self.ttl,
                },
            );
        }
        self.fanout(
            &channel_id,
            BufferEvent::Envelope(stored.clone()),
        )
        .await;
        Ok(stored)
    }

    /// Current unexpired envelope, or None if empty/expired (and clear if expired).
    pub async fn get(&self, channel_id: &str) -> Option<StoredEnvelope> {
        let now = self.now().await;
        let mut map = self.inner.write().await;
        match map.get(channel_id) {
            Some(state) if state.expires_at > now => Some(state.envelope.clone()),
            Some(_) => {
                map.remove(channel_id);
                None
            }
            None => None,
        }
    }

    /// Snapshot for a new subscriber: current envelope or empty.
    pub async fn snapshot_event(&self, channel_id: &str) -> BufferEvent {
        match self.get(channel_id).await {
            Some(env) => BufferEvent::Envelope(env),
            None => BufferEvent::Empty {
                channel_id: channel_id.to_string(),
            },
        }
    }

    /// Subscribe to live updates for a channel. Returns a receiver.
    pub async fn subscribe(&self, channel_id: &str) -> broadcast::Receiver<BufferEvent> {
        let mut subs = self.subscribers.write().await;
        if let Some(tx) = subs.get(channel_id) {
            return tx.subscribe();
        }
        let (tx, rx) = broadcast::channel(64);
        subs.insert(channel_id.to_string(), tx);
        rx
    }

    async fn fanout(&self, channel_id: &str, event: BufferEvent) {
        let subs = self.subscribers.read().await;
        if let Some(tx) = subs.get(channel_id) {
            let _ = tx.send(event);
        }
    }

    /// Inspect stored ciphertext buffers (integration: assert no plaintext).
    pub async fn debug_stored_ciphertexts(&self) -> Vec<Vec<u8>> {
        let now = self.now().await;
        let map = self.inner.read().await;
        map.values()
            .filter(|s| s.expires_at > now)
            .map(|s| s.envelope.ciphertext.clone())
            .collect()
    }

    /// All live stored envelopes (ciphertext fields only).
    pub async fn debug_stored_envelopes(&self) -> Vec<StoredEnvelope> {
        let now = self.now().await;
        let map = self.inner.read().await;
        map.values()
            .filter(|s| s.expires_at > now)
            .map(|s| s.envelope.clone())
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublishError {
    BadChannel,
    TooLarge,
}

fn is_valid_channel_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample_ct(n: u8) -> Vec<u8> {
        vec![n; 32]
    }

    #[tokio::test]
    async fn publish_replaces_previous_latest_only() {
        let buf = ChannelBuffer::new(Duration::from_secs(60));
        let ch = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();

        buf.publish(ch.clone(), 1, vec![1; 24], sample_ct(1))
            .await
            .unwrap();
        buf.publish(ch.clone(), 1, vec![2; 24], sample_ct(2))
            .await
            .unwrap();

        let got = buf.get(&ch).await.expect("buffered");
        assert_eq!(got.ciphertext, sample_ct(2));
        assert_eq!(got.nonce, vec![2; 24]);
        let all = buf.debug_stored_envelopes().await;
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn subscribe_snapshot_returns_current_or_empty() {
        let buf = ChannelBuffer::new(Duration::from_secs(60));
        let ch = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        match buf.snapshot_event(ch).await {
            BufferEvent::Empty { channel_id } => assert_eq!(channel_id, ch),
            other => panic!("expected empty, got {other:?}"),
        }

        buf.publish(ch.to_string(), 1, vec![9; 24], sample_ct(9))
            .await
            .unwrap();

        match buf.snapshot_event(ch).await {
            BufferEvent::Envelope(env) => {
                assert_eq!(env.ciphertext, sample_ct(9));
                assert_eq!(env.channel_id, ch);
            }
            other => panic!("expected envelope, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ttl_expiry_clears_buffer() {
        let start = Instant::now();
        let (buf, clock) = ChannelBuffer::with_manual_clock(Duration::from_millis(50), start);
        let ch = "cccccccccccccccccccccccccccccccc".to_string();

        buf.publish(ch.clone(), 1, vec![3; 24], sample_ct(3))
            .await
            .unwrap();
        assert!(buf.get(&ch).await.is_some());

        *clock.write().await = start + Duration::from_millis(51);
        assert!(buf.get(&ch).await.is_none());
        match buf.snapshot_event(&ch).await {
            BufferEvent::Empty { .. } => {}
            other => panic!("expected empty after TTL, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_oversized_ciphertext() {
        let buf = ChannelBuffer::new(Duration::from_secs(60));
        let ch = "dddddddddddddddddddddddddddddddd".to_string();
        let big = vec![0u8; MAX_CIPHERTEXT_BYTES + 1];
        let err = buf.publish(ch, 1, vec![0; 24], big).await.unwrap_err();
        assert_eq!(err, PublishError::TooLarge);
    }
}
