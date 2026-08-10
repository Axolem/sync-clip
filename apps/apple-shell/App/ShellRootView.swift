import AppleShellCore
import SwiftUI

/// Shared Shell settings UI for iOS, iPadOS, macOS (windowed), and visionOS.
public struct ShellRootView: View {
    @EnvironmentObject private var model: ShellModel

    public init() {}

    public var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text(model.statusMessage)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                Section("This Device") {
                    TextField("Local nickname", text: $model.nickname)
                    HStack {
                        Button("Save nickname") { model.saveNickname() }
                        Button("Clear", role: .destructive) { model.clearNickname() }
                    }
                }

                Section("Sync Group") {
                    TextField("Link Key (base32)", text: $model.linkKeyText, axis: .vertical)
                        .lineLimit(2...4)
                        #if os(iOS) || os(visionOS)
                        .textInputAutocapitalization(.characters)
                        .autocorrectionDisabled()
                        #endif
                    HStack {
                        Button("Generate key") { model.generateNewLinkKey() }
                        Button("Save / Join") { model.saveAndJoin() }
                            .buttonStyle(.borderedProminent)
                    }
                    Button("Rotate key", role: .destructive) { model.rotateKey() }
                    Toggle("Armed — sync clipboard", isOn: Binding(
                        get: { model.armed },
                        set: { model.setArmed($0) }
                    ))
                }

                Section("Relay") {
                    TextField("Relay WebSocket URL", text: $model.relayUrl)
                        #if os(iOS) || os(visionOS)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                        #endif
                }

                Section {
                    Text(
                        "Clipboard sync needs Sync Clip in the foreground on iPhone and iPad. On Mac, use the menu bar Shell (SyncClipMac / SyncClip Shell.app) for pasteboard capture with Shell Lifetime and login-item resume."
                    )
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Sync Clip")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.large)
            #endif
        }
    }
}
