import AppKit
import Foundation

/// Coordinates clipboard watch + Session publish/poll for the macOS Shell.
public final class ClipboardSyncController {
    private let echoGuard = PasteboardEchoGuard()
    private var lastChangeCount: Int
    private let pasteboard: NSPasteboard
    private var pollTimer: Timer?
    private var session: Session?
    private var watchTimer: Timer?

    public init(pasteboard: NSPasteboard = .general) {
        self.pasteboard = pasteboard
        self.lastChangeCount = pasteboard.changeCount
    }

    public var isArmed: Bool {
        session?.isArmed() ?? false
    }

    public func attach(session: Session) {
        self.session = session
        lastChangeCount = pasteboard.changeCount
        startLoops()
    }

    public func detach() {
        stopLoops()
        session = nil
    }

    public func setArmed(_ armed: Bool) {
        session?.setArmed(armed: armed)
    }

    public func stopLoops() {
        pollTimer?.invalidate()
        pollTimer = nil
        watchTimer?.invalidate()
        watchTimer = nil
    }

    private func startLoops() {
        stopLoops()
        watchTimer = Timer.scheduledTimer(withTimeInterval: 0.35, repeats: true) { [weak self] _ in
            self?.pollLocalPasteboard()
        }
        pollTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            self?.pollRemoteApplied()
        }
        if let watchTimer {
            RunLoop.main.add(watchTimer, forMode: .common)
        }
        if let pollTimer {
            RunLoop.main.add(pollTimer, forMode: .common)
        }
    }

    private func pollLocalPasteboard() {
        guard let session, session.isArmed() else { return }
        let changeCount = pasteboard.changeCount
        guard changeCount != lastChangeCount else { return }
        lastChangeCount = changeCount
        let text = pasteboard.string(forType: .string)
        if echoGuard.shouldIgnoreChange(currentText: text) {
            return
        }
        guard let text, !text.isEmpty else { return }
        do {
            try session.publishText(text: text)
        } catch {
            NSLog("sync-clip: publish failed: \(error)")
        }
    }

    private func pollRemoteApplied() {
        guard let session, session.isArmed() else { return }
        guard let applied = session.pollApplied() else { return }
        echoGuard.markRemoteWrite(text: applied.text)
        pasteboard.clearContents()
        pasteboard.setString(applied.text, forType: .string)
        lastChangeCount = pasteboard.changeCount
    }
}
