import ServiceManagement

/// Registers the macOS Shell for login auto-start when eligible (ADR-0006).
public enum LoginItemController {
    /// Best-effort: enable login item when auto-start should be on; disable otherwise.
    @discardableResult
    public static func syncLoginItem(shouldEnable: Bool) -> Bool {
        if #available(macOS 13.0, *) {
            let service = SMAppService.mainApp
            do {
                if shouldEnable {
                    try service.register()
                } else if service.status == .enabled {
                    try service.unregister()
                }
                return true
            } catch {
                NSLog("sync-clip: login item sync failed (soft): \(error)")
                return false
            }
        }
        return false
    }
}
