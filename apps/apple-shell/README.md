# Apple Shell (iOS, iPadOS, macOS menu bar, visionOS)

Shared Sync Clip Shell package: `AppleShellCore` (Clip Engine via UniFFI) plus:

- **macOS:** menu bar / login-item Shell (`SyncClipMac` → `SyncClip Shell.app`)
- **iOS / iPadOS / visionOS:** SwiftUI settings UI (`SyncClip.xcodeproj`)

## Prerequisites

- Xcode 15+
- Rust toolchain
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) for the mobile app project (`brew install xcodegen`)

## macOS menu bar

```bash
# from repo root
./scripts/build-macos-shell.sh
# → ~/Applications/SyncClip Shell.app
```

## iOS / visionOS

```bash
./scripts/build-apple-ffi.sh          # ClipFfi.xcframework
cd apps/apple-shell
xcodegen generate
open SyncClip.xcodeproj
```

## Tests

```bash
cd apps/apple-shell
# ensure lib/libclip_ffi.a exists (build-macos-shell or copy from target/)
swift test
```

## Platform notes

| Platform | Role |
|---|---|
| macOS menu bar | Shell Lifetime, login item, continuous pasteboard (`LSUIElement`) |
| iOS / iPadOS | Foreground UIPasteboard while the app is active |
| visionOS | UIPasteboard (same UIKit adapter); FFI slice when SDK present |

watchOS and tvOS are out of scope.
