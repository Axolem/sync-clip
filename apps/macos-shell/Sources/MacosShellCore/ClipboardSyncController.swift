import AppKit
import Foundation

/// Local clipboard snapshot read from NSPasteboard (Shell capture).
public struct LocalClipboardSnapshot: Equatable {
    public var imageBytes: Data?
    public var imageMime: String?
    public var text: String?

    public init(imageBytes: Data? = nil, imageMime: String? = nil, text: String? = nil) {
        self.imageBytes = imageBytes
        self.imageMime = imageMime
        self.text = text
    }

    public var isEmpty: Bool {
        let textEmpty = text?.isEmpty ?? true
        let imageEmpty = imageBytes?.isEmpty ?? true
        return textEmpty && imageEmpty
    }
}

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

    public var isSyncIdle: Bool {
        session?.isSyncIdle() ?? false
    }

    public var hasSession: Bool {
        session != nil
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
        let snapshot = readLocalSnapshot()
        if echoGuard.shouldIgnoreChange(currentText: snapshot.text) {
            return
        }
        guard !snapshot.isEmpty else { return }
        publish(snapshot: snapshot, session: session)
    }

    private func pollRemoteApplied() {
        guard let session, session.isArmed() else { return }
        guard let applied = session.pollApplied() else { return }
        echoGuard.markRemoteWrite(text: applied.text)
        writeApplied(applied)
        lastChangeCount = pasteboard.changeCount
    }

    private func publish(snapshot: LocalClipboardSnapshot, session: Session) {
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

    public func readLocalSnapshot() -> LocalClipboardSnapshot {
        let text = pasteboard.string(forType: .string)
        if let png = pasteboard.data(forType: .png), !png.isEmpty {
            return LocalClipboardSnapshot(imageBytes: png, imageMime: "image/png", text: text)
        }
        if let tiff = pasteboard.data(forType: .tiff), !tiff.isEmpty {
            return LocalClipboardSnapshot(imageBytes: tiff, imageMime: "image/tiff", text: text)
        }
        return LocalClipboardSnapshot(text: text)
    }

    private func writeApplied(_ applied: AppliedClipFfi) {
        pasteboard.clearContents()
        var wrote = false
        if let bytes = applied.imageBytes, let mime = applied.imageMime, !bytes.isEmpty {
            let type: NSPasteboard.PasteboardType =
                mime.contains("jpeg") || mime.contains("jpg") ? .init("public.jpeg") : .png
            wrote = pasteboard.setData(bytes, forType: type)
            if !wrote {
                wrote = pasteboard.setData(bytes, forType: .tiff)
            }
        }
        if !applied.text.isEmpty {
            pasteboard.setString(applied.text, forType: .string)
            wrote = true
        }
        if !wrote {
            pasteboard.setString(applied.text, forType: .string)
        }
    }
}
