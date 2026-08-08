import Foundation

/// Suppresses echo when the Shell writes a remote Clip to NSPasteboard.
///
/// After applying a remote Clip, the next observed pasteboard change that matches
/// the applied text (or any change while `ignoreRemaining` > 0) is treated as
/// Shell-authored and must not be published.
public final class PasteboardEchoGuard: @unchecked Sendable {
    private let lock = NSLock()
    private var ignoreRemaining: Int = 0
    private var lastAppliedText: String?

    public init() {}

    /// Call immediately before writing a remote Clip to the pasteboard.
    public func markRemoteWrite(text: String) {
        lock.lock()
        defer { lock.unlock() }
        ignoreRemaining += 1
        lastAppliedText = text
    }

    /// Returns true when this pasteboard change should not be published.
    public func shouldIgnoreChange(currentText: String?) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        if ignoreRemaining > 0 {
            ignoreRemaining -= 1
            if let currentText, currentText == lastAppliedText {
                return true
            }
            return true
        }
        if let currentText, let lastAppliedText, currentText == lastAppliedText {
            return true
        }
        return false
    }

    public func reset() {
        lock.lock()
        defer { lock.unlock() }
        ignoreRemaining = 0
        lastAppliedText = nil
    }
}
