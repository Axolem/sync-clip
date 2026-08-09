import Foundation

/// Local clipboard snapshot captured by a Shell (plain text and/or image).
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

/// Platform pasteboard seam used by `ClipboardSyncController`.
public protocol SystemClipboard: AnyObject {
    var changeCount: Int { get }
    func readSnapshot() -> LocalClipboardSnapshot
    func writeApplied(_ applied: AppliedClipFfi)
}
