import Foundation

/// macOS Shell skeleton for Sync Clip.
///
/// The Shell owns OS clipboard read/write, background lifetime, Link Key
/// storage, and UI. Sync behavior is delegated to the Clip Engine (linked
/// later over FFI). This target is a buildable stub only.
@main
struct MacosShell {
    static func main() {
        let version = "0.1.0"
        print("sync-clip macOS Shell \(version) — skeleton; Clip Engine not linked yet")
        print("States: Armed / Paused (not implemented)")
        print("Sync Group / Link Key: not configured")
    }
}
