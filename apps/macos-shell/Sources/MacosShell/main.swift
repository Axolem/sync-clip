import AppKit
import MacosShellCore

@main
enum MacosShellMain {
    static func main() {
        let app = NSApplication.shared
        let delegate = MenuBarShellApp()
        app.delegate = delegate
        // Accessory: no Dock icon (also set LSUIElement in Info.plist for .app bundles).
        app.setActivationPolicy(.accessory)
        app.run()
    }
}
