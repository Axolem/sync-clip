//! Shell Lifetime policy — pure rules for resume-on-boot, Arm, and capture gates.
//!
//! See ADR-0006 and CONTEXT.md (Shell Lifetime, Sync Idle, Elevated Clipboard Capture).

/// Snapshot of durable Shell state used for lifetime decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifetimeSnapshot {
    /// Durable Armed flag (persisted by the Shell).
    pub durable_armed: bool,
    /// Android Elevated Clipboard Capture granted; macOS should pass `true`.
    pub elevated_capture_granted: bool,
    /// Whether a Link Key is currently saved on this Device.
    pub has_link_key: bool,
    /// User Quit opted out of the next boot/login auto-start until reopen.
    pub quit_opted_out: bool,
    /// Platform requires Elevated Clipboard Capture to Arm (Android: true).
    pub requires_elevated_capture: bool,
}

/// Whether boot/login should auto-start the Shell.
pub fn may_auto_start(snapshot: &LifetimeSnapshot) -> bool {
    snapshot.has_link_key && snapshot.durable_armed && !snapshot.quit_opted_out
}

/// Whether the Device may enter Armed (Clip publish/accept).
pub fn may_enter_armed(snapshot: &LifetimeSnapshot) -> bool {
    if !snapshot.has_link_key {
        return false;
    }
    if snapshot.requires_elevated_capture && !snapshot.elevated_capture_granted {
        return false;
    }
    true
}

/// Boot auto-started with Armed intent but capture missing → persist Paused.
pub fn boot_should_force_paused(snapshot: &LifetimeSnapshot) -> bool {
    snapshot.has_link_key
        && snapshot.durable_armed
        && snapshot.requires_elevated_capture
        && !snapshot.elevated_capture_granted
}

/// Capture denied/revoked while requiring it → persist Paused.
pub fn capture_missing_should_persist_paused(
    requires_elevated_capture: bool,
    elevated_capture_granted: bool,
) -> bool {
    requires_elevated_capture && !elevated_capture_granted
}

/// Shell Lifetime continues only while a Link Key is saved.
pub fn should_keep_lifetime(has_link_key: bool) -> bool {
    has_link_key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> LifetimeSnapshot {
        LifetimeSnapshot {
            durable_armed: true,
            elevated_capture_granted: true,
            has_link_key: true,
            quit_opted_out: false,
            requires_elevated_capture: false,
        }
    }

    #[test]
    fn auto_start_requires_link_key_armed_and_not_quit() {
        assert!(may_auto_start(&base()));
        assert!(!may_auto_start(&LifetimeSnapshot {
            has_link_key: false,
            ..base()
        }));
        assert!(!may_auto_start(&LifetimeSnapshot {
            durable_armed: false,
            ..base()
        }));
        assert!(!may_auto_start(&LifetimeSnapshot {
            quit_opted_out: true,
            ..base()
        }));
    }

    #[test]
    fn arm_blocked_without_elevated_capture_on_android() {
        let android = LifetimeSnapshot {
            elevated_capture_granted: false,
            requires_elevated_capture: true,
            ..base()
        };
        assert!(!may_enter_armed(&android));
        assert!(may_enter_armed(&LifetimeSnapshot {
            elevated_capture_granted: true,
            ..android
        }));
        assert!(may_enter_armed(&base())); // macOS-style
    }

    #[test]
    fn boot_forces_paused_when_armed_but_capture_missing() {
        let snap = LifetimeSnapshot {
            elevated_capture_granted: false,
            requires_elevated_capture: true,
            ..base()
        };
        assert!(boot_should_force_paused(&snap));
        assert!(!boot_should_force_paused(&base()));
    }

    #[test]
    fn capture_revoke_persists_paused() {
        assert!(capture_missing_should_persist_paused(true, false));
        assert!(!capture_missing_should_persist_paused(true, true));
        assert!(!capture_missing_should_persist_paused(false, false));
    }

    #[test]
    fn clearing_link_key_ends_lifetime() {
        assert!(should_keep_lifetime(true));
        assert!(!should_keep_lifetime(false));
    }
}
