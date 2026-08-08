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
