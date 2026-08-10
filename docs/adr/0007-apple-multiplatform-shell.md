# Multiplatform Apple Shell (iOS / iPadOS / macOS menu bar / visionOS)

Apple platforms share `apps/apple-shell`: `AppleShellCore` + UniFFI `clip-ffi`.

## Decision

- **macOS menu bar** lives in `AppleShellCore` (`MenuBarShellApp`, `LoginItemController`) and ships as the `SyncClipMac` executable / `SyncClip Shell.app` via `scripts/build-macos-shell.sh` (bundle id `com.syncclip.shell`, `LSUIElement`).
- **iOS / iPadOS / visionOS** use the XcodeGen `SyncClip` app (`com.syncclip.shell.apple`) with SwiftUI `ShellRootView`.
- **`apps/macos-shell` is removed** — it duplicated the menu bar after the shared core existed (supersedes the “keep separate macos-shell” line in the original ADR-0007 text).
- iOS/iPadOS clipboard capture remains foreground-oriented via `UIPasteboard`.
- watchOS and tvOS stay out of scope.

## Consequences

- One Swift package owns Mac menu bar + mobile UI adapters.
- Keychain / nickname defaults for the menu bar keep `com.syncclip.macos-shell*` service keys so existing Mac installs keep Link Keys.
- CI builds `SyncClipMac` and tests `AppleShellCore` from `apps/apple-shell`.
