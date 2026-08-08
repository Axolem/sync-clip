//! WebSocket relay seam tests: latest-only publish + subscribe snapshot.

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use relay::{start_relay, RelayConfig};
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

async fn recv_json(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timeout")
        .expect("stream end")
        .expect("ws error");
    match msg {
        Message::Text(t) => serde_json::from_str(&t).expect("json"),
        other => panic!("unexpected frame {other:?}"),
    }
}

#[tokio::test]
async fn ws_subscribe_empty_then_publish_fans_out_latest_only() {
    let handle = start_relay(RelayConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], 0)),
        ttl: Duration::from_secs(60),
    })
    .await
    .unwrap();
    let url = handle.ws_url();
    let ch = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    let (mut a, _) = connect_async(&url).await.unwrap();
    a.send(Message::Text(
        format!(r#"{{"type":"subscribe","channel_id":"{ch}"}}"#).into(),
    ))
    .await
    .unwrap();
    let snap = recv_json(&mut a).await;
    assert_eq!(snap["type"], "empty");
    assert_eq!(snap["channel_id"], ch);

    let (mut b, _) = connect_async(&url).await.unwrap();
    b.send(Message::Text(
        format!(r#"{{"type":"subscribe","channel_id":"{ch}"}}"#).into(),
    ))
    .await
    .unwrap();
    let _ = recv_json(&mut b).await; // empty

    // Publish A then B — latest-only.
    a.send(Message::Text(
        format!(
            r#"{{"type":"publish","channel_id":"{ch}","protocol_version":1,"nonce":"{}","ciphertext":"{}"}}"#,
            b64(&[1; 24]),
            b64(b"cipher-A")
        )
        .into(),
    ))
    .await
    .unwrap();

    let env_a = recv_json(&mut b).await;
    assert_eq!(env_a["type"], "envelope");
    assert_eq!(env_a["ciphertext"], b64(b"cipher-A"));

    a.send(Message::Text(
        format!(
            r#"{{"type":"publish","channel_id":"{ch}","protocol_version":1,"nonce":"{}","ciphertext":"{}"}}"#,
            b64(&[2; 24]),
            b64(b"cipher-B")
        )
        .into(),
    ))
    .await
    .unwrap();

    let env_b = recv_json(&mut b).await;
    assert_eq!(env_b["type"], "envelope");
    assert_eq!(env_b["ciphertext"], b64(b"cipher-B"));

    // New subscriber gets latest snapshot only.
    let (mut c, _) = connect_async(&url).await.unwrap();
    c.send(Message::Text(
        format!(r#"{{"type":"subscribe","channel_id":"{ch}"}}"#).into(),
    ))
    .await
    .unwrap();
    let snap2 = recv_json(&mut c).await;
    assert_eq!(snap2["type"], "envelope");
    assert_eq!(snap2["ciphertext"], b64(b"cipher-B"));

    let stored = handle.buffer.debug_stored_envelopes().await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].ciphertext, b"cipher-B");
    // Relayed buffers hold ciphertext bytes only — not a plaintext Clip field.
    assert!(!String::from_utf8_lossy(&stored[0].ciphertext).contains("schema_version"));

    handle.shutdown();
}
