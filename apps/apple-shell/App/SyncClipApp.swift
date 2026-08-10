import AppleShellCore
import SwiftUI

@main
struct SyncClipApp: App {
    @StateObject private var model = ShellModel()

    var body: some Scene {
        WindowGroup {
            ShellRootView()
                .environmentObject(model)
                .onAppear { model.onAppear() }
        }
    }
}
