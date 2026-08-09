# Windows Shell

Tray Shell for Sync Clip on Windows. Uses `clip-ffi` `Session` (same Clip Engine facade as other Shells), JSON config under `%AppData%\SyncClip\`, and the system clipboard via `arboard`.

## Seams (tested)

- `LinkKeyStoring` / `FileCredentialStore`
- `ArmedStateStoring` (Armed + Quit opt-out, ADR-0006)
- `LocalNicknameStoring`
- `PasteboardEchoGuard`
- `SystemClipboard` + `ClipboardSyncController`

## Build

From the repo root (on Windows, or cross-check core on any OS):

```bash
# Core unit tests (macOS/Linux/Windows)
cargo test -p windows-shell

# Windows tray binary (Windows host or Windows target)
cargo build -p windows-shell --release
# → target/release/sync-clip-shell.exe
```

## Run (Windows)

```text
sync-clip-shell.exe
```

Tray menu:

- **Generate Link Key** — saves credentials, joins, copies Link Key to clipboard
- **Join with clipboard Link Key** — reads base32 Link Key from clipboard
- **Toggle Armed / Paused**
- **Set Local Nickname from clipboard**
- **Quit Sync Clip** — ends Shell Lifetime and opts out of the next auto-start (ADR-0006)

## Notes

- No Elevated Clipboard Capture on Windows; the Shell reads the clipboard while it is running (tray process).
- Auto-start on login is a follow-up (Run key / Task Scheduler when Armed + Link Key and not quit-opted-out).
- Credential file is local JSON (not DPAPI yet); treat the machine profile as the trust boundary for v1.
