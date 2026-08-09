# Distributing Sync Clip Shells

Primary channel: **GitHub Releases** (notarized macOS DMG + signed Android APK).  
Secondary prep: Play Console AAB + store listing assets; Mac App Store deferred (sandbox / login-item constraints).

Version is read from [`VERSION`](../VERSION) at the repo root.

## Credentials (local only)

Never commit keystores or `.p8` keys. Use [`secrets/`](../secrets/) (gitignored) and environment variables.

### Apple — Developer ID + notarization

```bash
security find-identity -v -p codesigning | grep "Developer ID Application"
```

| Variable | Purpose |
|---|---|
| `SYNC_CLIP_CODESIGN_IDENTITY` | Full identity string from `find-identity` |
| `SYNC_CLIP_TEAM_ID` | 10-char Team ID (needed for Apple ID notary auth) |
| `SYNC_CLIP_NOTARY_KEY_PATH` | Path to App Store Connect API `.p8` |
| `SYNC_CLIP_NOTARY_KEY_ID` | Key ID |
| `SYNC_CLIP_NOTARY_ISSUER_ID` | Issuer ID |
| *or* `SYNC_CLIP_APPLE_ID` + `SYNC_CLIP_APP_SPECIFIC_PASSWORD` | Alternate notary auth |

Create an API key: [App Store Connect](https://appstoreconnect.apple.com) → Users and Access → Integrations → Team Keys.

### Android — release signing

```bash
mkdir -p secrets
keytool -genkeypair -v -keystore secrets/sync-clip-upload.jks \
  -alias sync-clip -keyalg RSA -keysize 2048 -validity 10000
```

| Variable | Purpose |
|---|---|
| `SYNC_CLIP_ANDROID_KEYSTORE` | Absolute path to `.jks` / `.keystore` |
| `SYNC_CLIP_ANDROID_STORE_PASSWORD` | Keystore password |
| `SYNC_CLIP_ANDROID_KEY_ALIAS` | Key alias |
| `SYNC_CLIP_ANDROID_KEY_PASSWORD` | Key password |

If unset, `assembleRelease` falls back to the **debug** keystore and prints a warning — do not publish those APKs.

## Package locally

```bash
# Android APK (+ optional AAB for Play prep)
./scripts/package-android-shell.sh
BUNDLE=1 ./scripts/package-android-shell.sh

# macOS DMG (codesign + notarize when env is set)
./scripts/package-macos-shell.sh
# Local smoke without notary:
NOTARIZE=0 ./scripts/package-macos-shell.sh
```

Artifacts land in `dist/`:

- `SyncClip-Shell-<version>-macos.dmg` (+ `.sha256`)
- `sync-clip-shell-<version>.apk` (+ `.sha256`)
- `sync-clip-shell-<version>.aab` (+ `.sha256`) when `BUNDLE=1`

### Verify macOS Gatekeeper

```bash
spctl --assess -vv --type install dist/SyncClip-Shell-*-macos.dmg
xcrun stapler validate dist/SyncClip-Shell-*-macos.dmg
```

### Install Android APK

```bash
adb install -r dist/sync-clip-shell-<version>.apk
```

Then enable **Settings → Accessibility → Sync Clip** before Arming.

## GitHub Release (wave 1)

1. Bump [`VERSION`](../VERSION) and matching notes.
2. Package both Shells locally (macOS notarization stays on your machine in wave 1).
3. Tag and push: `git tag v$(cat VERSION) && git push origin v$(cat VERSION)`.
4. CI builds the Android release APK on the tag (see [`.github/workflows/release-shells.yml`](../.github/workflows/release-shells.yml)).
5. Create the GitHub Release for that tag; attach:
   - notarized `SyncClip-Shell-*-macos.dmg` (+ `.sha256`) from your machine
   - CI (or local) `sync-clip-shell-*.apk` (+ `.sha256`)
6. Paste short notes from [`assets/store/LISTING.md`](../assets/store/LISTING.md).

CI does **not** notarize the macOS DMG yet (cert import TBD). Upload the DMG you produced with `package-macos-shell.sh`.

## Play Console (wave 2 prep)

- Listing copy + graphics: [`assets/store/LISTING.md`](../assets/store/LISTING.md)
- Build AAB: `BUNDLE=1 ./scripts/package-android-shell.sh`
- Keep Accessibility disclosure in the listing; Play remains non-primary while Elevated Clipboard Capture is required ([ADR-0006](adr/0006-shell-lifetime-and-elevated-capture.md)).

## Mac App Store (wave 2 notes)

Not packaged in this wave. Expect friction:

- App Sandbox vs pasteboard + outbound WebSocket relay
- Login item / `SMAppService` vs MAS rules
- Menu bar accessory apps need careful review notes (`LSUIElement`)

Prefer Developer ID + notarized DMG via GitHub until MAS entitlements are designed.

The multiplatform SwiftUI Shell (`apps/apple-shell`) targets iOS / iPadOS / macOS windowed / visionOS for TestFlight / App Store later; see [ADR-0007](adr/0007-apple-multiplatform-shell.md).

## Bundle identity

| Shell | Bundle / application id |
|---|---|
| macOS menu bar | `com.syncclip.shell` |
| Apple multiplatform (iOS / iPadOS / macOS windowed / visionOS) | `com.syncclip.shell.apple` |
| Android | `com.syncclip.shell` |

Display name: **Sync Clip**.
