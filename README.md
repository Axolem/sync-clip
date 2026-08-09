# Sync Clip

Cross-device clipboard sync: Devices in a Sync Group exchange Clips so a copy on one Device can be pasted on another with normal OS paste.

This monorepo holds the shared **Clip Engine** (Rust), the **clip-ffi** Session facade (UniFFI), the encrypted **relay**, and native **Shells** (macOS, Android). See [CONTEXT.md](CONTEXT.md) and [docs/adr/](docs/adr/) for domain language and architecture decisions.

## Layout

```
crates/clip-engine/   # Clip Engine library (Rust)
crates/clip-ffi/      # Blocking Session facade + UniFFI (Swift/Kotlin)
crates/relay/         # Encrypted relay (WebSocket, ciphertext only)
apps/macos-shell/     # macOS menu bar Shell (SwiftPM + UniFFI)
apps/apple-shell/     # iOS / iPadOS / macOS windowed / visionOS Shell (SwiftUI + UniFFI)
apps/windows-shell/   # Windows tray Shell (Rust + clip-ffi)
apps/android-shell/   # Android Shell (Gradle / Kotlin + UniFFI)
scripts/              # UniFFI generate + platform FFI helpers
```

## Prerequisites

- Rust toolchain (`rustc` / `cargo`) — [rustup](https://rustup.rs/)
- Swift 5.9+ / Xcode Command Line Tools (macOS Shell)
- JDK 17+, Android SDK + NDK (Android Shell). Set `ANDROID_HOME` (e.g. `~/Library/Android/sdk` on macOS)

Default Shell relay URL is **`wss://clip.dotenv.co.za/v0/ws`** (`clip-ffi` `DEFAULT_RELAY_WS_URL`). Override in Shell settings for local/self-hosted relays.

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

Local relay for development (optional — Shells default to the hosted URL):

```bash
cargo run -p relay -- --bind 127.0.0.1:7120
```

Local WebSocket endpoint: `ws://127.0.0.1:7120/v0/ws` (ciphertext envelopes only; see `docs/protocol/clip-wire-v0.md`).

### Docker (hosted relay)

The relay is ciphertext-only and has no accounts. Put TLS in front with your reverse proxy if you want `wss://`.

**CI** builds the image on GitHub Actions and publishes to GHCR on `main`:

```bash
docker pull ghcr.io/axolem/sync-clip/relay:latest
docker run --rm -p 7120:7120 -e RUST_LOG=info ghcr.io/axolem/sync-clip/relay:latest
```

If the package is private, authenticate first:

```bash
echo "$GH_TOKEN" | docker login ghcr.io -u USERNAME --password-stdin
```

Local build + run:

```bash
# Build + run (port 7120 on the host)
docker compose up -d --build

# Or plain Docker
docker build -t sync-clip-relay .
docker run --rm -p 7120:7120 -e RUST_LOG=info sync-clip-relay
```

Optional env (compose): `RELAY_PORT` (host port, default `7120`), `RELAY_TTL_SECS` (default `900`), `RUST_LOG`. Use `IMAGE=ghcr.io/axolem/sync-clip/relay:latest docker compose up -d` to run the published image without building.

Point Shells at (or keep the built-in default):

- `wss://clip.dotenv.co.za/v0/ws` (hosted default)
- `ws://<host>:7120/v0/ws` (plain, LAN/VPS without TLS)
- `wss://relay.example.com/v0/ws` (other self-hosted + TLS)

Example Caddy snippet:

```caddy
relay.example.com {
  reverse_proxy localhost:7120
}
```

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

### Apple Shell (iOS / iPadOS / macOS windowed / visionOS)

```bash
./scripts/build-apple-ffi.sh          # ClipFfi.xcframework + macOS libclip_ffi.a
cd apps/apple-shell
xcodegen generate                     # SyncClip.xcodeproj
open SyncClip.xcodeproj               # Run on Simulator / device / Mac / visionOS
swift test                            # AppleShellCore unit tests (macOS)
```

See [`apps/apple-shell/README.md`](apps/apple-shell/README.md) and [ADR-0007](docs/adr/0007-apple-multiplatform-shell.md). The menu bar Shell remains the macOS path for Shell Lifetime with login-item resume.

### Windows Shell

```bash
cargo test -p windows-shell
cargo build -p windows-shell --release   # on Windows → sync-clip-shell.exe
```

Tray Shell: Generate / join Link Key (clipboard), Armed toggle, Local Nickname, Quit opt-out ([ADR-0008](docs/adr/0008-windows-shell.md)).

### Android Shell

```bash
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
./scripts/build-android-jni.sh   # builds libclip_ffi.so into jniLibs
cd apps/android-shell
./gradlew assembleDebug
./gradlew installDebug   # optional
```

`assembleDebug` succeeds without JNI libs (Kotlin-only packaging); clipboard sync at runtime requires `./scripts/build-android-jni.sh` first.

## Distribute (GitHub Releases)

Version comes from [`VERSION`](VERSION). Store listing copy and graphics live in [`assets/store/`](assets/store/). Full credentials checklist: [`docs/distribution.md`](docs/distribution.md).

```bash
# Android release APK → dist/sync-clip-shell-<ver>.apk
./scripts/package-android-shell.sh
# Optional Play-prep bundle:
BUNDLE=1 ./scripts/package-android-shell.sh

# macOS notarized DMG → dist/SyncClip-Shell-<ver>-macos.dmg
# Requires SYNC_CLIP_CODESIGN_IDENTITY + notary env (see docs/distribution.md)
./scripts/package-macos-shell.sh
```

Tag `v*` pushes build the Android APK via GitHub Actions and attach it to the Release. Upload the notarized macOS DMG from your machine in wave 1.

Day-to-day local install (no DMG):

```bash
./scripts/build-macos-shell.sh   # ~/Applications/SyncClip Shell.app
```

## Demo: macOS + Android on one Sync Group

1. Relay: use the hosted default (`wss://clip.dotenv.co.za/v0/ws`), or start a local relay and set Relay URL in each Shell.
2. macOS: `swift run` from `apps/macos-shell` (after copying `libclip_ffi.a` as above).
   - Menu → **Generate Link Key** (or Enter Link Key). Leave **Armed** on.
   - Optional: **Relay URL…**, **Rotate Link Key…**, **Local Nickname…**
3. Android: install debug APK with JNI libs built.
   - Paste the same Link Key; relay defaults to `defaultRelayWsUrl()` (`wss://clip.dotenv.co.za/v0/ws`). **Save / Join**, keep **Armed**.
   - Optional: Local Nickname, **Rotate Link Key**, edit relay URL then Save / Join.
4. Copy plain text or an in-cap image (~5 MiB encoded max) on one Device; paste on the other.
   - Text + oversized image → text syncs; image is omitted.

Pause on either Shell stops publish and accept but **keeps Shell Lifetime** (relay stay joined; Android FGS stays up). Quit on macOS ends the process and opts out of login auto-start until you open the Shell again. Unreachable relays are **Sync Idle** (Armed stays on; reconnect retries). Boot/login auto-start runs when a Link Key is saved and the Device was last Armed.

**Android Elevated Clipboard Capture:** enable the Sync Clip Accessibility service before Arming (sideload/APK builds). Without it, Arm is blocked and revoke persists Paused.

Remote clipboard writes are echo-suppressed (Engine + Shell ignore flags). Link Key rotation replaces the Sync Group credential on that Device; Devices still on the old key no longer exchange Clips with rotators. Local Nickname is UI-only and never enters Clip plaintext or the relay.

## Vocabulary

Use **Clip Engine**, **Shell**, **Shell Lifetime**, **Sync Group**, **Link Key**, **Armed** / **Paused**, **Sync Idle**, **Elevated Clipboard Capture** as defined in [CONTEXT.md](CONTEXT.md). See [ADR-0006](docs/adr/0006-shell-lifetime-and-elevated-capture.md), [ADR-0007](docs/adr/0007-apple-multiplatform-shell.md), [ADR-0008](docs/adr/0008-windows-shell.md).
