# Sync Clip

Cross-device clipboard sync: Devices in a Sync Group exchange Clips so a copy on one Device can be pasted on another with normal OS paste.

This monorepo holds the shared **Clip Engine** (Rust), the **clip-ffi** Session facade (UniFFI), the encrypted **relay**, and native **Shells** (macOS, Android). See [CONTEXT.md](CONTEXT.md) and [docs/adr/](docs/adr/) for domain language and architecture decisions.

## Layout

```
crates/clip-engine/   # Clip Engine library (Rust)
crates/clip-ffi/      # Blocking Session facade + UniFFI (Swift/Kotlin)
crates/relay/         # Encrypted relay (WebSocket, ciphertext only)
apps/macos-shell/     # macOS menu bar Shell (SwiftPM + UniFFI)
apps/android-shell/   # Android Shell (Gradle / Kotlin + UniFFI)
scripts/              # UniFFI generate + Android JNI helpers
```

## Prerequisites

- Rust toolchain (`rustc` / `cargo`) — [rustup](https://rustup.rs/)
- Swift 5.9+ / Xcode Command Line Tools (macOS Shell)
- JDK 17+, Android SDK + NDK (Android Shell). Set `ANDROID_HOME` (e.g. `~/Library/Android/sdk` on macOS)

## Build

### Clip Engine + relay + FFI (Cargo workspace)

```bash
source "$HOME/.cargo/env"   # if needed
cargo build --workspace
cargo test --workspace
```

### Regenerate UniFFI bindings (after changing `crates/clip-ffi`)

```bash
./scripts/generate-uniffi.sh
```

### Relay CLI

Default demo bind for Shells is port **7120** (matches `clip-ffi` `DEFAULT_RELAY_WS_URL`):

```bash
cargo run -p relay -- --bind 127.0.0.1:7120
```

WebSocket endpoint: `ws://127.0.0.1:7120/v0/ws` (ciphertext envelopes only; see `docs/protocol/clip-wire-v0.md`).

### macOS Shell

```bash
source "$HOME/.cargo/env"
cargo build -p clip-ffi
mkdir -p apps/macos-shell/lib
cp target/debug/libclip_ffi.a apps/macos-shell/lib/libclip_ffi.a
cd apps/macos-shell
swift build
swift run MacosShell   # menu bar extra; accessory (no Dock)
```

`Info.plist` sets `LSUIElement=true` for bundled runs; `swift run` also calls `NSApp.setActivationPolicy(.accessory)`.

### Android Shell

```bash
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
./scripts/build-android-jni.sh   # builds libclip_ffi.so into jniLibs
cd apps/android-shell
./gradlew assembleDebug
./gradlew installDebug   # optional
```

`assembleDebug` succeeds without JNI libs (Kotlin-only packaging); clipboard sync at runtime requires `./scripts/build-android-jni.sh` first.

## Demo: macOS + Android on one Sync Group

1. Start the relay: `cargo run -p relay -- --bind 127.0.0.1:7120`
2. macOS: `swift run` from `apps/macos-shell` (after copying `libclip_ffi.a` as above).
   - Menu → **Generate Link Key** (or Enter Link Key). Leave **Armed** on.
   - Optional: **Relay URL…**, **Rotate Link Key…**, **Local Nickname…**
3. Android: install debug APK with JNI libs built.
   - Paste the same Link Key, confirm relay from `defaultRelayWsUrl()` (use `10.0.2.2:7120` for emulator → host), **Save / Join**, keep **Armed**.
   - Optional: Local Nickname, **Rotate Link Key**, edit relay URL then Save / Join.
4. Copy plain text or an in-cap image (~5 MiB encoded max) on one Device; paste on the other.
   - Text + oversized image → text syncs; image is omitted.

Pause on either Shell stops publish and accept; Arm resumes. Unreachable relays fail soft (Armed stays on; sync idle). Remote clipboard writes are echo-suppressed (Engine + Shell ignore flags). Link Key rotation replaces the Sync Group credential on that Device; Devices still on the old key no longer exchange Clips with rotators. Local Nickname is UI-only and never enters Clip plaintext or the relay.

## Vocabulary

Use **Clip Engine**, **Shell**, **Sync Group**, **Link Key**, **Armed** / **Paused** as defined in [CONTEXT.md](CONTEXT.md). Avoid synonyms such as “core library”, “client”, “room”, or “pairing code”.
