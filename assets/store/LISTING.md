# Sync Clip — store listing copy

Use with assets in this directory. Android remains **sideload / APK primary** while Elevated Clipboard Capture (Accessibility) is required to Arm ([ADR-0006](../../docs/adr/0006-shell-lifetime-and-elevated-capture.md)). Play Console fields below are prepared for a future listing; GitHub Releases is the current install path.

## Assets

| Asset | File | Spec |
|---|---|---|
| Logo mark | `logo-mark.png` | 1024×1024 |
| App icon | `app-icon-1024.png` / `app-icon-512.png` | Apple 1024² / Play 512² |
| Feature graphic | `feature-graphic-1024x500.png` | Play 1024×500 |
| Promo hero | `promo-hero.png` | Marketing |
| Open Graph | `og-1200x630.png` | 1200×630 |

## Title

**Sync Clip**

## Subtitle / short description (≤80 chars)

Clipboard sync across your devices — copy once, paste anywhere.

## Full description

Sync Clip keeps your clipboard in sync across Mac and Android.

Copy text or an image on one device; paste with the normal OS paste on another. Devices join a Sync Group with a shared Link Key — no accounts. Clips are end-to-end encrypted; the relay only sees ciphertext.

**What you get**

- Cross-device clipboard sync (plain text and images)
- Armed / Paused control — pause sync without leaving the Sync Group
- Always-on Shell Lifetime while a Link Key is saved
- Optional local nickname (stays on the device)

**Android note**

Elevated Clipboard Capture (Accessibility service) is required to Arm so copies sync while other apps are focused. Grant it in Settings → Accessibility → Sync Clip. Sideload / GitHub APK builds are the supported install path today.

**Privacy in one line**

Link Keys and Clip contents are not used as identity; the relay cannot read Clip plaintext.

## Keywords (App Store style)

clipboard, sync, paste, cross-device, clipboard manager, share clipboard, mac, android

## Category

Utilities / Productivity

## Privacy / permission disclosures

- **Clipboard**: read and write the system clipboard to sync Clips.
- **Network**: connect to your chosen relay (default hosted `wss://` endpoint) with encrypted envelopes only.
- **Notifications (Android)**: foreground service status while Shell Lifetime is up.
- **Accessibility (Android)**: Elevated Clipboard Capture — required to Arm; used only to observe clipboard-relevant events for sync, not to drive other apps’ UI.
- **Login item (macOS)**: optional open at login when Armed with a saved Link Key.
- **Battery exemption (Android)**: optional request so background sync is not killed aggressively.

## Support / marketing URLs (fill when publishing)

- Homepage:
- Support:
- Privacy policy:
- GitHub Releases: https://github.com/Axolem/sync-clip/releases

## Review notes (Play / MAS — wave 2)

- Explain Accessibility: required for background clipboard capture on Android; deny leaves the device Paused.
- macOS is a menu bar (accessory) app (`LSUIElement`); no Dock icon by design.
- Default relay is ciphertext-only; users may point Shells at a self-hosted relay.
