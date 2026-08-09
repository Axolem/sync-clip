import Foundation

/// Suppresses echo when the Shell writes a remote Clip to the system clipboard.
public final class PasteboardEchoGuard: @unchecked Sendable {
    private let lock = NSLock()
    private var ignoreRemaining: Int = 0
    private var lastAppliedText: String?

    public init() {}

    public func markRemoteWrite(text: String) {
        lock.lock()
        defer { lock.unlock() }
        ignoreRemaining += 1
        lastAppliedText = text
    }

    public func shouldIgnoreChange(currentText: String?) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        if ignoreRemaining > 0 {
            ignoreRemaining -= 1
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
