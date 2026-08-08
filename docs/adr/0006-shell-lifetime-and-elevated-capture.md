# Shell Lifetime, resume-on-boot, and Elevated Clipboard Capture

Shell Lifetime stays up whenever a Link Key is saved: Paused keeps Sync Group membership and the running Shell; only publish/accept stop. Boot/login auto-start runs only with a saved Link Key and durable Armed. Quit ends the process and opts out of the next auto-start until the user opens the Shell again (Armed is preserved; opening re-enables auto-start when Armed). Soft-fail is Sync Idle with background retry, not Pause. OS vetoes (Force Stop, revoked login item) leave the Shell down until the user opens it.

On Android, Elevated Clipboard Capture is required to Arm (Arm is blocked until granted). Deny/revoke persists Paused; boot with Armed intent but missing capture auto-starts then forces Paused and notifies. macOS uses normal pasteboard only. Android stays sideload/APK-primary while elevated capture is required. OS resume permissions (login item, battery exemptions) are requested on first successful join while Armed.

## Considered Options

- Tear down FGS/process on Pause (previous Android behavior) — rejected; Pause must not end Shell Lifetime.
- Auto-start whenever a Link Key exists — rejected; Paused Devices should not resurrect Armed listening after reboot.
- Accept focus-only Android clipboard capture — rejected; local publish must work in background via Elevated Clipboard Capture.
- Stay Armed when capture is missing (remote-only) — rejected; without capture the Device is forced Paused until the user Arms again after granting it.
- Play Store as primary Android channel with elevated capture — rejected for now; sideload first.

## Consequences

- Armed/Paused must be persisted on both Shells so resume-on-boot is consistent.
- Android Pause no longer stops Shell Lifetime; macOS Quit remains the intentional stop that opts out of auto-start.
- Glossary: Shell Lifetime, Sync Idle, and Elevated Clipboard Capture (see `CONTEXT.md`).
