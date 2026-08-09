# Multiplatform Apple Shell (iOS / iPadOS / macOS windowed / visionOS)

Add a SwiftUI Apple Shell beside the existing macOS menu bar Shell so Sync Clip runs on iPhone, iPad, Mac (windowed), and visionOS, sharing `AppleShellCore` and UniFFI `clip-ffi`.

## Decision

- Ship **`apps/apple-shell`**: `AppleShellCore` (SPM) + XcodeGen multiplatform app (`supportedDestinations: iOS, macOS, visionOS`).
- Keep **`apps/macos-shell`** as the menu bar / login-item Shell for macOS pasteboard capture with Shell Lifetime (ADR-0006).
- iOS/iPadOS clipboard capture is **foreground-oriented** via `UIPasteboard`. There is no Android-style Elevated Clipboard Capture; background observation is best-effort while the process is awake.
- watchOS and tvOS are out of scope (no useful clipboard sync UX).
- Link `clip-ffi` through `ClipFfi.xcframework` for Xcode; macOS `swift test` uses `lib/libclip_ffi.a`.
- Bundle id: multiplatform app uses `com.syncclip.shell.apple`; menu bar keeps `com.syncclip.shell`.
- visionOS destination is declared; `build-apple-ffi.sh` adds `xros` slices when the visionOS SDK / Rust targets are installed (otherwise iOS+macOS only).

## Consequences

- Glossary: document that Elevated Clipboard Capture is Android-only; Apple mobile Shells rely on foreground pasteboard access.
- Store / TestFlight packaging for the Apple Shell is a follow-up (GitHub Releases wave can add an IPA later).
- Menu bar and windowed Mac apps are separate binaries with distinct bundle ids in the `com.syncclip.shell*` family.
