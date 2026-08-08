//! Integration: two Sessions + local relay exchange plain-text Clips (FFI seam).

use clip_ffi::{
    generate_ephemeral_id, generate_link_key, link_key_from_base32, link_key_to_base32, Session,
};
use relay::{start_relay, RelayConfig};
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

fn start_test_relay() -> (relay::RelayHandle, String) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let handle = rt
        .block_on(start_relay(RelayConfig {
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ttl: Duration::from_secs(60),
        }))
        .expect("start relay");
    let url = handle.ws_url();
    // Keep the runtime alive for the relay task.
    std::mem::forget(rt);
    (handle, url)
}

fn poll_until(session: &Session, timeout: Duration) -> Option<clip_ffi::AppliedClipFfi> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if let Some(applied) = session.poll_applied() {
            return Some(applied);
        }
        thread::sleep(Duration::from_millis(20));
    }
    None
}

#[test]
fn link_key_round_trips_base32_without_padding() {
    let key = generate_link_key();
    assert_eq!(key.len(), 32);
    let encoded = link_key_to_base32(key.clone());
    assert!(!encoded.contains('='), "base32 must be unpadded");
    let decoded = link_key_from_base32(encoded).expect("decode");
    assert_eq!(decoded, key);
}

#[test]
fn two_sessions_exchange_plain_text_clip() {
    let (relay, url) = start_test_relay();
    let link_key = generate_link_key();
    let a = Session::new(link_key.clone(), url.clone(), generate_ephemeral_id())
        .expect("session A");
    let b = Session::new(link_key, url, generate_ephemeral_id()).expect("session B");
    thread::sleep(Duration::from_millis(80));

    assert!(a.is_armed());
    assert!(b.is_armed());

    a.publish_text("hello from session A".into())
        .expect("publish");
    let applied = poll_until(&b, Duration::from_secs(3)).expect("B should apply");
    assert_eq!(applied.text, "hello from session A");
    assert!(!applied.id_hex.is_empty());

    relay.shutdown();
}

#[test]
fn two_sessions_exchange_in_cap_image_clip() {
    let (relay, url) = start_test_relay();
    let link_key = generate_link_key();
    let a = Session::new(link_key.clone(), url.clone(), generate_ephemeral_id())
        .expect("session A");
    let b = Session::new(link_key, url, generate_ephemeral_id()).expect("session B");
    thread::sleep(Duration::from_millis(80));

    let png = b"\x89PNG\r\nffi-image".to_vec();
    a.publish_text_and_image("ffi caption".into(), png.clone(), "image/png".into())
        .expect("publish");
    let applied = poll_until(&b, Duration::from_secs(3)).expect("B should apply");
    assert_eq!(applied.text, "ffi caption");
    assert_eq!(applied.image_mime.as_deref(), Some("image/png"));
    assert_eq!(applied.image_bytes.as_ref(), Some(&png));

    relay.shutdown();
}

#[test]
fn session_over_cap_image_omits_image_keeps_text() {
    let (relay, url) = start_test_relay();
    let link_key = generate_link_key();
    let a = Session::new(link_key.clone(), url.clone(), generate_ephemeral_id())
        .expect("session A");
    let b = Session::new(link_key, url, generate_ephemeral_id()).expect("session B");
    thread::sleep(Duration::from_millis(80));

    let oversized = vec![0u8; clip_engine::MAX_IMAGE_BYTES + 1];
    a.publish_text_and_image("keep text".into(), oversized, "image/png".into())
        .expect("publish");
    let applied = poll_until(&b, Duration::from_secs(3)).expect("B applies text");
    assert_eq!(applied.text, "keep text");
    assert!(applied.image_bytes.is_none());
    assert!(applied.image_mime.is_none());

    relay.shutdown();
}

#[test]
fn sessions_on_rotated_link_key_do_not_sync_with_old_key() {
    let (relay, url) = start_test_relay();
    let old_key = generate_link_key();
    let new_key = generate_link_key();
    let a = Session::new(new_key.clone(), url.clone(), generate_ephemeral_id()).expect("A");
    let b = Session::new(new_key, url.clone(), generate_ephemeral_id()).expect("B");
    let c = Session::new(old_key, url, generate_ephemeral_id()).expect("C old");
    thread::sleep(Duration::from_millis(80));

    a.publish_text("new group only".into()).expect("publish");
    let applied = poll_until(&b, Duration::from_secs(3)).expect("B applies");
    assert_eq!(applied.text, "new group only");
    assert!(
        poll_until(&c, Duration::from_millis(400)).is_none(),
        "old Link Key Session must not receive new-group Clips"
    );

    relay.shutdown();
}

#[test]
fn paused_session_neither_publishes_nor_applies_until_armed() {
    let (relay, url) = start_test_relay();
    let link_key = generate_link_key();
    let a = Session::new(link_key.clone(), url.clone(), generate_ephemeral_id())
        .expect("session A");
    let b = Session::new(link_key, url, generate_ephemeral_id()).expect("session B");
    thread::sleep(Duration::from_millis(80));

    b.set_armed(false);
    assert!(!b.is_armed());
    assert!(b.publish_text("from paused".into()).is_err());

    a.publish_text("while B paused".into()).expect("A publish");
    assert!(
        poll_until(&b, Duration::from_millis(400)).is_none(),
        "Paused B must not apply"
    );

    b.set_armed(true);
    a.publish_text("after B armed".into()).expect("A publish");
    let applied = poll_until(&b, Duration::from_secs(3)).expect("Armed B applies");
    assert_eq!(applied.text, "after B armed");

    relay.shutdown();
}
