//! Encrypted relay for Sync Clip — opaque envelope WebSocket server.

use clap::Parser;
use relay::{start_relay, RelayConfig, DEFAULT_TTL};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "relay",
    about = "Encrypted relay for Clip delivery (ciphertext only)",
    version
)]
struct Args {
    /// Bind address (default 127.0.0.1:8787).
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: SocketAddr,

    /// Buffer TTL in seconds (default 900 = 15 minutes).
    #[arg(long, default_value_t = 900)]
    ttl_secs: u64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let ttl = if args.ttl_secs == 900 {
        DEFAULT_TTL
    } else {
        Duration::from_secs(args.ttl_secs)
    };
    let handle = start_relay(RelayConfig {
        bind: args.bind,
        ttl,
    })
    .await
    .expect("bind relay");

    tracing::info!("sync-clip relay listening on {}", handle.ws_url());
    // Park forever; process exit stops the server.
    std::future::pending::<()>().await;
}
