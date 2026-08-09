#if canImport(UIKit)
import Foundation
import UIKit
import UniformTypeIdentifiers

/// UIPasteboard-backed clipboard for iOS, iPadOS, and visionOS.
///
/// Capture runs while the Shell is in the foreground (or briefly active).
/// iOS does not offer Android-style Elevated Clipboard Capture; background
/// observation is best-effort via change count when the process is awake.
public final class UIKitSystemClipboard: SystemClipboard {
    private let pasteboard: UIPasteboard

    public init(pasteboard: UIPasteboard = .general) {
        self.pasteboard = pasteboard
    }

    public var changeCount: Int {
        pasteboard.changeCount
    }

    public func readSnapshot() -> LocalClipboardSnapshot {
        let text = pasteboard.string
        if let data = pasteboard.data(forPasteboardType: UTType.png.identifier), !data.isEmpty {
            return LocalClipboardSnapshot(imageBytes: data, imageMime: "image/png", text: text)
        }
        if let data = pasteboard.data(forPasteboardType: UTType.jpeg.identifier), !data.isEmpty {
            return LocalClipboardSnapshot(imageBytes: data, imageMime: "image/jpeg", text: text)
        }
        if let image = pasteboard.image, let data = image.pngData(), !data.isEmpty {
            return LocalClipboardSnapshot(imageBytes: data, imageMime: "image/png", text: text)
        }
        return LocalClipboardSnapshot(text: text)
    }

    public func writeApplied(_ applied: AppliedClipFfi) {
        var items: [String: Any] = [:]
        if !applied.text.isEmpty {
            items[UTType.utf8PlainText.identifier] = applied.text
        }
        if let bytes = applied.imageBytes, let mime = applied.imageMime, !bytes.isEmpty {
            if mime.contains("jpeg") || mime.contains("jpg") {
                items[UTType.jpeg.identifier] = bytes
            } else {
                items[UTType.png.identifier] = bytes
            }
        }
        if items.isEmpty {
            pasteboard.string = applied.text
        } else {
            pasteboard.setItems([items])
        }
    }
}
#endif
