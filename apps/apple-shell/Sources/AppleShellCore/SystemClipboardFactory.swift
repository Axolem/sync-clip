import Foundation

/// Factory for the platform system clipboard.
public enum SystemClipboardFactory {
    public static func makeDefault() -> SystemClipboard {
        #if canImport(AppKit) && !targetEnvironment(macCatalyst)
        return AppKitSystemClipboard()
        #elseif canImport(UIKit)
        return UIKitSystemClipboard()
        #else
        fatalError("No SystemClipboard adapter for this Apple platform")
        #endif
    }
}
