import AppleShellCore
import SwiftUI

@main
struct SyncClipApp: App {
    @StateObject private var model = ShellModel()
    #if os(macOS)
    @NSApplicationDelegateAdaptor(MacAppDelegate.self) private var macDelegate
    #endif

    var body: some Scene {
        WindowGroup {
            ShellRootView()
                .environmentObject(model)
                .onAppear {
                    #if os(macOS)
                    macDelegate.model = model
                    #endif
                    model.onAppear()
                }
        }
        #if os(macOS)
        .defaultSize(width: 420, height: 560)
        #endif
    }
}

#if os(macOS)
import AppKit

/// Marks intentional Quit for ADR-0006 opt-out on the windowed Mac Shell.
final class MacAppDelegate: NSObject, NSApplicationDelegate {
    weak var model: ShellModel?

    func applicationWillTerminate(_ notification: Notification) {
        model?.prepareForTermination()
    }
}
#endif
