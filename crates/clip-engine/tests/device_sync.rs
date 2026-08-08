//! Integration: two Devices + local relay exchange plain-text Clips.

use clip_engine::{AppliedClip, Device, LinkKey};
use futures_util::{SinkExt, StreamExt};
use relay::{start_relay, RelayConfig};
use std::net::SocketAddr;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

fn link_key() -> LinkKey {
    LinkKey([0x7A; 32])
}

async fn start_test_relay() -> relay::RelayHandle {
    start_relay(RelayConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], 0)),
        ttl: Duration::from_secs(60),
    })
    .await
    .expect("start relay")
}

async fn join_pair(url: &str) -> (Device, Device) {
    let a = Device::join(link_key(), url, [0xA1; 16])
        .await
        .expect("device A");
    let b = Device::join(link_key(), url, [0xB2; 16])
        .await
        .expect("device B");
    // Allow subscribe handshake to complete.
    tokio::time::sleep(Duration::from_millis(50)).await;
    (a, b)
}

async fn next_applied_timeout(
    device: &mut Device,
    ms: u64,
) -> Result<AppliedClip, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_millis(ms), device.next_applied_clip())
        .await
        .map(|r| r.expect("device closed"))
}

#[tokio::test]
async fn two_devices_exchange_plain_text_clip_ciphertext_only_on_relay() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    let plaintext = "paste me on the other Device";
    a.publish_text(plaintext, 1_700_000_000_000)
        .await
        .expect("publish");

    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("B should apply");
    assert_eq!(applied.text, plaintext);

    // Plaintext never appears in relay buffer stores (ciphertext fields only).
    let stored = relay.buffer.debug_stored_envelopes().await;
    assert_eq!(stored.len(), 1);
    let ct = &stored[0].ciphertext;
    let lossy = String::from_utf8_lossy(ct);
    assert!(
        !lossy.contains(plaintext),
        "relay ciphertext must not contain plaintext"
    );
    assert!(!lossy.contains("paste me"));
    assert!(!lossy.contains("schema_version"));
    assert!(!lossy.contains("sender_ephemeral_id"));

    relay.shutdown();
}

#[tokio::test]
async fn paused_device_neither_publishes_nor_applies_until_armed() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    b.set_armed(false).await.expect("pause B");
    assert!(!b.is_armed());

    assert!(matches!(
        b.publish_text("from paused", 1).await,
        Err(clip_engine::DeviceError::Paused)
    ));

    a.publish_text("while B paused", 1_700_000_000_100)
        .await
        .expect("A publish");
    assert!(
        next_applied_timeout(&mut b, 300).await.is_err(),
        "Paused B must not apply"
    );

    b.set_armed(true).await.expect("arm B");
    a.publish_text("after B armed", 1_700_000_000_200)
        .await
        .expect("A publish again");
    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("Armed B applies");
    assert_eq!(applied.text, "after B armed");

    relay.shutdown();
}

#[tokio::test]
async fn echo_suppression_does_not_rebroadcast_applied_clip() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    let id = a
        .publish_text("echo candidate", 1_700_000_000_300)
        .await
        .expect("publish");
    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("B applies");
    assert_eq!(applied.id, id);

    let before = relay.buffer.debug_stored_envelopes().await;
    let nonce_before = before[0].nonce.clone();

    // Shell-sim: B "copies" the applied Clip back as a local publish.
    let echoed = b
        .publish_text_with_id("echo candidate", applied.created_at, applied.id)
        .await
        .expect("echo publish returns ok but suppressed");
    assert_eq!(echoed, applied.id);

    tokio::time::sleep(Duration::from_millis(100)).await;
    let after = relay.buffer.debug_stored_envelopes().await;
    assert_eq!(after[0].nonce, nonce_before, "relay must not get a new envelope");

    // A must not observe a new distinct applied Clip from the echo.
    assert!(next_applied_timeout(&mut a, 300).await.is_err());

    relay.shutdown();
}

#[tokio::test]
async fn lww_discards_older_created_at_when_newer_already_applied() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    a.publish_text("newer", 1_700_000_000_500)
        .await
        .expect("newer");
    let first = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("apply newer");
    assert_eq!(first.text, "newer");

    // Older Clip published later on the wire — Engine must discard via LWW.
    a.publish_text("older", 1_700_000_000_400)
        .await
        .expect("older");
    assert!(
        next_applied_timeout(&mut b, 400).await.is_err(),
        "older created_at must not apply after newer"
    );

    relay.shutdown();
}

#[tokio::test]
async fn lww_tie_break_prefers_greater_id_bytewise() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    let ts = 1_700_000_000_600_i64;
    let low_id = [0x01; 16];
    let high_id = [0xFF; 16];

    a.publish_text_with_id("low", ts, low_id)
        .await
        .expect("low");
    let first = next_applied_timeout(&mut b, 2_000).await.expect("apply low");
    assert_eq!(first.id, low_id);

    a.publish_text_with_id("high", ts, high_id)
        .await
        .expect("high");
    let second = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("apply high");
    assert_eq!(second.id, high_id);
    assert_eq!(second.text, "high");

    // Reverse order would not apply lower id after higher.
    a.publish_text_with_id("low-again", ts, low_id)
        .await
        .expect("low again");
    assert!(next_applied_timeout(&mut b, 400).await.is_err());

    relay.shutdown();
}

#[tokio::test]
async fn in_cap_image_roundtrips_between_devices() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    let png = b"\x89PNG\r\nin-cap-image".to_vec();
    a.publish("caption", Some((png.clone(), "image/png".into())), 1_700_000_001_000)
        .await
        .expect("publish image");

    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("B should apply image Clip");
    assert_eq!(applied.text, "caption");
    let image = applied.image.expect("image present");
    assert_eq!(image.mime, "image/png");
    assert_eq!(image.bytes, png);

    relay.shutdown();
}

#[tokio::test]
async fn over_cap_image_is_omitted_but_text_still_syncs() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let (mut a, mut b) = join_pair(&url).await;

    let oversized = vec![0xAB; clip_engine::MAX_IMAGE_BYTES + 1];
    a.publish(
        "text survives",
        Some((oversized, "image/png".into())),
        1_700_000_001_100,
    )
    .await
    .expect("publish with over-cap image");

    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("B should apply text-only Clip");
    assert_eq!(applied.text, "text survives");
    assert!(applied.image.is_none(), "over-cap image must be omitted");

    relay.shutdown();
}

#[tokio::test]
async fn rotated_link_key_isolates_old_sync_group() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let old_key = LinkKey([0x11; 32]);
    let new_key = LinkKey([0x22; 32]);

    let mut a = Device::join(new_key, &url, [0xA1; 16])
        .await
        .expect("A new key");
    let mut b = Device::join(new_key, &url, [0xB2; 16])
        .await
        .expect("B new key");
    let mut c = Device::join(old_key, &url, [0xC3; 16])
        .await
        .expect("C old key");
    tokio::time::sleep(Duration::from_millis(50)).await;

    a.publish_text("only new Sync Group", 1_700_000_001_200)
        .await
        .expect("publish");

    let applied = next_applied_timeout(&mut b, 2_000)
        .await
        .expect("B on new key applies");
    assert_eq!(applied.text, "only new Sync Group");
    assert!(
        next_applied_timeout(&mut c, 400).await.is_err(),
        "C on old Link Key must not receive new-group Clips"
    );

    relay.shutdown();
}

/// Extra check: raw WS observer sees only ciphertext fields on the wire.
#[tokio::test]
async fn wire_observer_never_sees_clip_plaintext_fields() {
    let relay = start_test_relay().await;
    let url = relay.ws_url();
    let ch = clip_engine::channel_id_hex(&link_key());

    let (mut observer, _) = connect_async(&url).await.unwrap();
    observer
        .send(Message::Text(
            format!(r#"{{"type":"subscribe","channel_id":"{ch}"}}"#).into(),
        ))
        .await
        .unwrap();
    let _empty = observer.next().await;

    let (mut a, mut b) = join_pair(&url).await;
    a.publish_text("secret-plaintext-xyz", 1_700_000_000_700)
        .await
        .unwrap();
    let _ = next_applied_timeout(&mut b, 2_000).await.unwrap();

    let frame = tokio::time::timeout(Duration::from_secs(2), observer.next())
        .await
        .expect("timeout")
        .expect("end")
        .expect("err");
    let Message::Text(text) = frame else {
        panic!("expected text");
    };
    assert!(text.contains("ciphertext"));
    assert!(!text.contains("secret-plaintext-xyz"));
    assert!(!text.contains("schema_version"));
    assert!(!text.contains("sender_ephemeral_id"));

    relay.shutdown();
}
