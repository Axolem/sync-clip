//! Encrypted relay skeleton for Sync Clip.
//!
//! Shells will connect using material derived from the Link Key; the relay
//! only stores and forwards ciphertext. This binary is a buildable stub —
//! no sync protocol yet.

use clap::Parser;
use clip_engine;

#[derive(Parser, Debug)]
#[command(
    name = "relay",
    about = "Encrypted relay skeleton for Clip delivery (no protocol yet)",
    version
)]
struct Args {
    /// Print Clip Engine version and exit.
    #[arg(long)]
    engine_version: bool,
}

fn main() {
    let args = Args::parse();

    if args.engine_version {
        println!("{}", clip_engine::version());
        return;
    }

    // Skeleton only: bind nothing and exit cleanly after --help / default run.
    println!(
        "sync-clip relay {} (Clip Engine {}) — skeleton; no listeners yet",
        env!("CARGO_PKG_VERSION"),
        clip_engine::version()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn clip_engine_ping() {
        assert_eq!(clip_engine::ping(), "pong");
    }
}
