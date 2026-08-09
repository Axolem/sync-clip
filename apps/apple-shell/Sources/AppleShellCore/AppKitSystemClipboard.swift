#if canImport(AppKit) && !targetEnvironment(macCatalyst)
import AppKit
import Foundation

/// NSPasteboard-backed clipboard for native macOS Shells.
public final class AppKitSystemClipboard: SystemClipboard {
    private let pasteboard: NSPasteboard

    public init(pasteboard: NSPasteboard = .general) {
        self.pasteboard = pasteboard
    }

    public var changeCount: Int {
        pasteboard.changeCount
    }

    public func readSnapshot() -> LocalClipboardSnapshot {
        let text = pasteboard.string(forType: .string)
        if let png = pasteboard.data(forType: .png), !png.isEmpty {
            return LocalClipboardSnapshot(imageBytes: png, imageMime: "image/png", text: text)
        }
        if let tiff = pasteboard.data(forType: .tiff), !tiff.isEmpty {
            return LocalClipboardSnapshot(imageBytes: tiff, imageMime: "image/tiff", text: text)
        }
        return LocalClipboardSnapshot(text: text)
    }

    public func writeApplied(_ applied: AppliedClipFfi) {
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
#endif
