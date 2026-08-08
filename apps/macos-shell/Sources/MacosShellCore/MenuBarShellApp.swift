import AppKit
import Foundation

/// Menu bar Shell for Sync Clip (accessory / no Dock icon).
@MainActor
public final class MenuBarShellApp: NSObject, NSApplicationDelegate {
    private let clipboard = ClipboardSyncController()
    private var isArmed = true
    private let nicknameStore: LocalNicknameStoring
    private let store: LinkKeyStoring
    private var statusItem: NSStatusItem?
    private var syncIdleReason: String?

    public init(
        store: LinkKeyStoring = KeychainLinkKeyStore(),
        nicknameStore: LocalNicknameStoring = UserDefaultsLocalNicknameStore()
    ) {
        self.nicknameStore = nicknameStore
        self.store = store
    }

    public func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        refreshStatusTitle()
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
            syncIdleReason = "Could not restore session: \(error)"
            NSLog("sync-clip: failed to restore Link Key (soft): \(error)")
            refreshStatusTitle()
            rebuildMenu()
        }
    }

    private func refreshStatusTitle() {
        let nickname = nicknameStore.load()
        if let button = statusItem?.button {
            button.title = nickname.map { "Clip · \($0)" } ?? "Clip"
            var tip = "Sync Clip"
            if let syncIdleReason {
                tip += " — \(syncIdleReason)"
            }
            button.toolTip = tip
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
        menu.addItem(NSMenuItem(
            title: "Rotate Link Key…",
            action: #selector(onRotateLinkKey),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: "Relay URL…",
            action: #selector(onEditRelayUrl),
            keyEquivalent: ""
        ))
        menu.addItem(NSMenuItem(
            title: "Local Nickname…",
            action: #selector(onEditNickname),
            keyEquivalent: ""
        ))
        menu.addItem(NSMenuItem(
            title: "Clear Local Nickname",
            action: #selector(onClearNickname),
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
        if let syncIdleReason {
            let idle = NSMenuItem(
                title: "Sync idle: \(syncIdleReason)",
                action: nil,
                keyEquivalent: ""
            )
            idle.isEnabled = false
            menu.addItem(idle)
        }
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
        let relay = currentRelayUrl()
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
            softFailJoin(error: error)
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
            softFailJoin(error: error)
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

    @objc private func onRotateLinkKey() {
        let alert = NSAlert()
        alert.messageText = "Rotate Link Key"
        alert.informativeText =
            "Generate a new Link Key (clears the old one on this Device) or paste a replacement."
        alert.addButton(withTitle: "Generate New")
        alert.addButton(withTitle: "Adopt Pasted…")
        alert.addButton(withTitle: "Cancel")
        let response = alert.runModal()
        if response == .alertThirdButtonReturn { return }
        do {
            let existing = try store.load()
            let ephemeral = existing?.ephemeralId ?? generateEphemeralId()
            let relay = existing?.relayWsUrl ?? defaultRelayWsUrl()
            let newKey: Data
            if response == .alertFirstButtonReturn {
                newKey = generateLinkKey()
            } else {
                let paste = NSAlert()
                paste.messageText = "Adopt Link Key"
                paste.informativeText = "Paste the replacement Sync Group Link Key (base32)."
                paste.addButton(withTitle: "Adopt")
                paste.addButton(withTitle: "Cancel")
                let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 280, height: 24))
                paste.accessoryView = field
                guard paste.runModal() == .alertFirstButtonReturn else { return }
                newKey = try linkKeyFromBase32(encoded: field.stringValue)
            }
            try store.delete()
            let credentials = ShellCredentials(
                ephemeralId: ephemeral,
                linkKey: newKey,
                relayWsUrl: relay
            )
            try store.save(credentials)
            try join(credentials: credentials)
            presentAlert(
                title: "Link Key rotated",
                message: linkKeyToBase32(key: newKey)
            )
        } catch {
            softFailJoin(error: error)
        }
    }

    @objc private func onEditRelayUrl() {
        let alert = NSAlert()
        alert.messageText = "Relay URL"
        alert.informativeText = "WebSocket URL for the encrypted relay. Changing rejoins without a new Link Key."
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 320, height: 24))
        field.stringValue = currentRelayUrl()
        alert.accessoryView = field
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let url = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return }
        do {
            guard var credentials = try store.load() else {
                presentAlert(
                    title: "No Link Key",
                    message: "Generate or enter a Link Key before setting a relay URL."
                )
                return
            }
            credentials.relayWsUrl = url
            try store.save(credentials)
            try join(credentials: credentials)
        } catch {
            softFailJoin(error: error)
        }
    }

    @objc private func onEditNickname() {
        let alert = NSAlert()
        alert.messageText = "Local Nickname"
        alert.informativeText = "Stored only on this Device for UI. Never sent to the Sync Group or relay."
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 240, height: 24))
        field.stringValue = nicknameStore.load() ?? ""
        alert.accessoryView = field
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        nicknameStore.save(field.stringValue)
        refreshStatusTitle()
        rebuildMenu()
    }

    @objc private func onClearNickname() {
        nicknameStore.clear()
        refreshStatusTitle()
        rebuildMenu()
    }

    @objc private func onToggleArmed() {
        isArmed.toggle()
        clipboard.setArmed(isArmed)
        rebuildMenu()
    }

    @objc private func onQuit() {
        NSApp.terminate(nil)
    }

    private func currentRelayUrl() -> String {
        if let credentials = try? store.load() {
            return credentials.relayWsUrl
        }
        return defaultRelayWsUrl()
    }

    private func join(credentials: ShellCredentials) throws {
        guard credentials.linkKey.count == 32 else {
            throw SessionError.InvalidLinkKey
        }
        clipboard.detach()
        let session = try Session(
            linkKeyBytes: credentials.linkKey,
            relayWsUrl: credentials.relayWsUrl,
            ephemeralIdBytes: credentials.ephemeralId
        )
        session.setArmed(armed: isArmed)
        clipboard.attach(session: session)
        syncIdleReason = nil
        refreshStatusTitle()
        rebuildMenu()
    }

    private func softFailJoin(error: Error) {
        clipboard.detach()
        syncIdleReason = "\(error)"
        NSLog("sync-clip: join/publish soft-fail: \(error)")
        refreshStatusTitle()
        rebuildMenu()
        presentAlert(title: "Sync idle", message: "Staying Armed locally. \(error)")
    }

    private func presentAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.runModal()
    }
}
