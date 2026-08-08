# Sync Clip

Cross-device clipboard sync: Devices in a Sync Group exchange Clips so a copy on one Device can be pasted on another with normal OS paste.

This monorepo holds the shared **Clip Engine** (Rust), the encrypted **relay**, and native **Shells** (macOS, Android). See [CONTEXT.md](CONTEXT.md) and [docs/adr/](docs/adr/) for domain language and architecture decisions.

## Layout

```
crates/clip-engine/   # Clip Engine library (Rust)
crates/relay/         # Encrypted relay (WebSocket, ciphertext only)
apps/macos-shell/     # macOS Shell (Swift Package)
apps/android-shell/   # Android Shell (Gradle / Kotlin)
```

## Prerequisites

- Rust toolchain (`rustc` / `cargo`) — [rustup](https://rustup.rs/)
- Swift 5.9+ / Xcode Command Line Tools (macOS Shell)
- JDK 17+, Android SDK (Android Shell). Set `ANDROID_HOME` (e.g. `~/Library/Android/sdk` on macOS)

## Build

### Clip Engine + relay (Cargo workspace)

```bash
source "$HOME/.cargo/env"   # if needed
cargo build --workspace
cargo test --workspace
```

Relay CLI:

```bash
cargo run -p relay -- --help
cargo run -p relay -- --bind 127.0.0.1:8787
```

WebSocket endpoint: `ws://<bind>/v0/ws` (ciphertext envelopes only; see `docs/protocol/clip-wire-v0.md`).

### macOS Shell

```bash
cd apps/macos-shell
swift build
```

### Android Shell

```bash
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
cd apps/android-shell
./gradlew assembleDebug
```

## Vocabulary

Use **Clip Engine**, **Shell**, **Sync Group**, **Link Key**, **Armed** / **Paused** as defined in [CONTEXT.md](CONTEXT.md). Avoid synonyms such as “core library”, “client”, “room”, or “pairing code”.
