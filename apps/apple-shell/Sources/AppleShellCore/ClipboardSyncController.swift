import Foundation

/// Coordinates clipboard watch + Session publish/poll for Apple Shells.
public final class ClipboardSyncController {
    private let clipboard: SystemClipboard
    private let echoGuard = PasteboardEchoGuard()
    private var lastChangeCount: Int
    private var pollTimer: Timer?
    private var session: ClipSessioning?
    private var watchTimer: Timer?

    public init(clipboard: SystemClipboard) {
        self.clipboard = clipboard
        self.lastChangeCount = clipboard.changeCount
    }

    public var isArmed: Bool {
        session?.isArmed() ?? false
    }

    public var isSyncIdle: Bool {
        session?.isSyncIdle() ?? false
    }

    public var hasSession: Bool {
        session != nil
    }

    public func attach(session: ClipSessioning) {
        self.session = session
        lastChangeCount = clipboard.changeCount
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

    /// Single-step local watch (tests + timer).
    public func pollLocalClipboard() {
        guard let session, session.isArmed() else { return }
        let changeCount = clipboard.changeCount
        guard changeCount != lastChangeCount else { return }
        lastChangeCount = changeCount
        let snapshot = clipboard.readSnapshot()
        if echoGuard.shouldIgnoreChange(currentText: snapshot.text) {
            return
        }
        guard !snapshot.isEmpty else { return }
        publish(snapshot: snapshot, session: session)
    }

    /// Single-step remote poll (tests + timer).
    public func pollRemoteApplied() {
        guard let session, session.isArmed() else { return }
        guard let applied = session.pollApplied() else { return }
        echoGuard.markRemoteWrite(text: applied.text)
        clipboard.writeApplied(applied)
        lastChangeCount = clipboard.changeCount
    }

    private func startLoops() {
        stopLoops()
        watchTimer = Timer.scheduledTimer(withTimeInterval: 0.35, repeats: true) { [weak self] _ in
            self?.pollLocalClipboard()
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

    private func publish(snapshot: LocalClipboardSnapshot, session: ClipSessioning) {
        let text = snapshot.text ?? ""
        do {
            if let imageBytes = snapshot.imageBytes, let mime = snapshot.imageMime {
                try session.publishTextAndImage(
                    text: text,
                    imageBytes: imageBytes,
                    imageMime: mime
                )
            } else if !text.isEmpty {
                try session.publishText(text: text)
            }
        } catch {
            NSLog("sync-clip: publish failed (soft): \(error)")
        }
    }
}
