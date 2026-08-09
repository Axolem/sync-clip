import Foundation

/// Session seam for Shell sync loops (UniFFI `Session` + test doubles).
public protocol ClipSessioning: AnyObject {
    func isArmed() -> Bool
    func isSyncIdle() -> Bool
    func pollApplied() -> AppliedClipFfi?
    func publishText(text: String) throws
    func publishTextAndImage(text: String, imageBytes: Data, imageMime: String) throws
    func setArmed(armed: Bool)
}

extension Session: ClipSessioning {}
