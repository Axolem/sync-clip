# Windows Shell

Add a Windows tray Shell so Devices on Windows can join a Sync Group and sync Clips, using the same `clip-ffi` Session facade as Android and Apple Shells.

## Decision

- Ship **`apps/windows-shell`** as a Rust workspace crate (`sync-clip-shell.exe`) with:
  - shared core library (stores, echo guard, `SystemClipboard`, `ClipboardSyncController`) tested on all hosts
  - Windows-only tray UI (`tray-icon` + `tao`) and `arboard` clipboard adapter
- Persist Link Key / Armed / Local Nickname under `%AppData%\SyncClip\` (JSON v1).
- Apply ADR-0006 Quit opt-out when the tray Quit action ends the process.
- watchOS-style elevated capture is N/A; clipboard observation requires the Shell process to be running.

## Consequences

- CI runs `cargo test -p windows-shell` on Ubuntu/macOS (core) and `cargo build -p windows-shell` on `windows-latest`.
- Login auto-start and DPAPI/credential-manager hardening are follow-ups.
- Glossary Device/Shell language applies; avoid calling the tray a “client” or “peer”.
