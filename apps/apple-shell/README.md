# Apple Shell (iOS, iPadOS, macOS windowed, visionOS)

Multiplatform Sync Clip Shell built with SwiftUI + `AppleShellCore` (Clip Engine via UniFFI).

The **menu bar** macOS Shell remains in [`../macos-shell`](../macos-shell) for Shell Lifetime with login-item resume and continuous pasteboard capture. This app is the settings/UI Shell for iPhone, iPad, Vision Pro, and an optional Mac window.

## Prerequisites

- Xcode 15+
- Rust toolchain + targets: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-apple-darwin`
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`)

## Build Clip FFI

```bash
# from repo root
./scripts/build-apple-ffi.sh
```

Produces `lib/ClipFfi.xcframework` and `lib/libclip_ffi.a` (macOS SPM tests).

## Generate Xcode project

```bash
cd apps/apple-shell
xcodegen generate
open SyncClip.xcodeproj
```

Select the **Sync Clip** scheme → iOS Simulator / device / visionOS / My Mac.

## Tests (shared core)

```bash
./scripts/build-macos-shell.sh   # ensures libclip_ffi.a exists, or:
cp ../../target/release/libclip_ffi.a lib/
swift test
```

## Platform notes

| Platform | Clipboard capture |
|---|---|
| iOS / iPadOS | Foreground (and brief active) UIPasteboard; no Elevated Clipboard Capture equivalent |
| visionOS | UIPasteboard (same adapter) |
| macOS (this app) | NSPasteboard while the windowed app is running |
| macOS menu bar | [`../macos-shell`](../macos-shell) — preferred for Shell Lifetime / login item |

watchOS and tvOS are out of scope (no useful clipboard sync UX).
