import AppKit
import Foundation

/// Menu bar Shell for Sync Clip (accessory / no Dock icon).
@MainActor
public final class MenuBarShellApp: NSObject, NSApplicationDelegate {
    private let clipboard = ClipboardSyncController()
    private var isArmed = true
    private let store: LinkKeyStoring
    private var statusItem: NSStatusItem?

    public init(store: LinkKeyStoring = KeychainLinkKeyStore()) {
        self.store = store
    }

    public func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem?.button {
            button.title = "Clip"
            button.toolTip = "Sync Clip"
        }
        rebuildMenu()
        bootstrapSession()
    }

    public func applicationWillTerminate(_ notification: Notification) {
        clipboard.detach()
    }

    private func bootstrapSession() {
        do {
            if let credentials = try store.load() {
                try join(credentials: credentials)
            }
        } catch {
            NSLog("sync-clip: failed to restore Link Key: \(error)")
        }
    }

    private func rebuildMenu() {
        let menu = NSMenu()
        menu.addItem(NSMenuItem(
            title: "Generate Link Key",
            action: #selector(onGenerateLinkKey),
            keyEquivalent: ""
        ))
        menu.addItem(NSMenuItem(
            title: "Enter Link Key…",
            action: #selector(onEnterLinkKey),
            keyEquivalent: ""
        ))
        menu.addItem(NSMenuItem(
            title: "Show Link Key",
            action: #selector(onShowLinkKey),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        let armedItem = NSMenuItem(
            title: isArmed ? "Armed" : "Paused",
            action: #selector(onToggleArmed),
            keyEquivalent: ""
        )
        armedItem.state = isArmed ? .on : .off
        menu.addItem(armedItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: "Quit Sync Clip",
            action: #selector(onQuit),
            keyEquivalent: "q"
        ))
        for item in menu.items where item.action != nil {
            item.target = self
        }
        statusItem?.menu = menu
    }

    @objc private func onGenerateLinkKey() {
        let linkKey = generateLinkKey()
        let ephemeral = generateEphemeralId()
        let relay = defaultRelayWsUrl()
        let credentials = ShellCredentials(
            ephemeralId: ephemeral,
            linkKey: linkKey,
            relayWsUrl: relay
        )
        do {
            try store.save(credentials)
            try join(credentials: credentials)
            presentAlert(
                title: "Link Key generated",
                message: linkKeyToBase32(key: linkKey)
            )
        } catch {
            presentAlert(title: "Could not join Sync Group", message: "\(error)")
        }
    }

    @objc private func onEnterLinkKey() {
        let alert = NSAlert()
        alert.messageText = "Enter Link Key"
        alert.informativeText = "Paste the Sync Group Link Key (base32)."
        alert.addButton(withTitle: "Join")
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 280, height: 24))
        alert.accessoryView = field
        let response = alert.runModal()
        guard response == .alertFirstButtonReturn else { return }
        do {
            let key = try linkKeyFromBase32(encoded: field.stringValue)
            let ephemeral: Data
            let relay: String
            if let existing = try store.load() {
                ephemeral = existing.ephemeralId
                relay = existing.relayWsUrl
            } else {
                ephemeral = generateEphemeralId()
                relay = defaultRelayWsUrl()
            }
            let credentials = ShellCredentials(
                ephemeralId: ephemeral,
                linkKey: key,
                relayWsUrl: relay
            )
            try store.save(credentials)
            try join(credentials: credentials)
        } catch {
            presentAlert(title: "Invalid Link Key", message: "\(error)")
        }
    }

    @objc private func onShowLinkKey() {
        do {
            guard let credentials = try store.load() else {
                presentAlert(title: "No Link Key", message: "Generate or enter a Link Key first.")
                return
            }
            presentAlert(
                title: "Link Key",
                message: linkKeyToBase32(key: credentials.linkKey)
            )
        } catch {
            presentAlert(title: "Keychain error", message: "\(error)")
        }
    }

    @objc private func onToggleArmed() {
        isArmed.toggle()
        clipboard.setArmed(isArmed)
        rebuildMenu()
    }

    @objc private func onQuit() {
        NSApp.terminate(nil)
    }

    private func join(credentials: ShellCredentials) throws {
        clipboard.detach()
        let session = try Session(
            linkKeyBytes: credentials.linkKey,
            relayWsUrl: credentials.relayWsUrl,
            ephemeralIdBytes: credentials.ephemeralId
        )
        session.setArmed(armed: isArmed)
        clipboard.attach(session: session)
        rebuildMenu()
    }

    private func presentAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.runModal()
    }
}
