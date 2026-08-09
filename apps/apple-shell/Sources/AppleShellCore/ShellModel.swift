import Combine
import Foundation

/// Observable Shell facade for SwiftUI: Link Key, Armed, relay, nickname, sync status.
@MainActor
public final class ShellModel: ObservableObject {
    public enum Status: Equatable {
        case ready
        case joined
        case syncIdle(String)
        case error(String)
    }

    @Published public var armed: Bool
    @Published public var linkKeyText: String = ""
    @Published public var nickname: String = ""
    @Published public var relayUrl: String
    @Published public var status: Status = .ready
    @Published public var statusMessage: String = "Ready"

    private let armedStore: ArmedStateStoring
    private let clipboard: ClipboardSyncController
    private let nicknameStore: LocalNicknameStoring
    private var statusTimer: Timer?
    private let store: LinkKeyStoring

    public init(
        store: LinkKeyStoring = KeychainLinkKeyStore(),
        nicknameStore: LocalNicknameStoring = UserDefaultsLocalNicknameStore(),
        armedStore: ArmedStateStoring = UserDefaultsArmedStateStore(),
        clipboard: SystemClipboard = SystemClipboardFactory.makeDefault()
    ) {
        self.store = store
        self.nicknameStore = nicknameStore
        self.armedStore = armedStore
        self.clipboard = ClipboardSyncController(clipboard: clipboard)
        self.armed = armedStore.isArmed
        self.relayUrl = defaultRelayWsUrl()
        self.nickname = nicknameStore.load() ?? ""
    }

    public func onAppear() {
        armedStore.clearQuitOptOut()
        restoreFields()
        bootstrapSession()
        startStatusPoll()
    }

    public func onDisappear() {
        // Soft background — do not treat as Quit opt-out (iOS suspension ≠ Quit).
        statusTimer?.invalidate()
        statusTimer = nil
    }

    public func prepareForTermination() {
        armedStore.quitOptedOut = true
        clipboard.detach()
        statusTimer?.invalidate()
        statusTimer = nil
    }

    public func saveNickname() {
        nicknameStore.save(nickname)
        refreshStatusMessage()
    }

    public func clearNickname() {
        nicknameStore.clear()
        nickname = ""
        refreshStatusMessage()
    }

    public func generateNewLinkKey() {
        let key = generateLinkKey()
        linkKeyText = linkKeyToBase32(key: key)
        if relayUrl.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            relayUrl = defaultRelayWsUrl()
        }
    }

    public func saveAndJoin() {
        do {
            let key = try linkKeyFromBase32(encoded: linkKeyText)
            let ephemeral: Data
            if let existing = try store.load() {
                ephemeral = existing.ephemeralId
            } else {
                ephemeral = generateEphemeralId()
            }
            let relay = normalizedRelay(relayUrl)
            let credentials = ShellCredentials(
                ephemeralId: ephemeral,
                linkKey: key,
                relayWsUrl: relay
            )
            try store.save(credentials)
            relayUrl = relay
            try join(credentials: credentials)
        } catch {
            status = .error("\(error)")
            statusMessage = "Join failed: \(error)"
        }
    }

    public func rotateKey() {
        do {
            let key = generateLinkKey()
            linkKeyText = linkKeyToBase32(key: key)
            let ephemeral = try store.load()?.ephemeralId ?? generateEphemeralId()
            let relay = normalizedRelay(relayUrl)
            let credentials = ShellCredentials(
                ephemeralId: ephemeral,
                linkKey: key,
                relayWsUrl: relay
            )
            try store.delete()
            try store.save(credentials)
            try join(credentials: credentials)
        } catch {
            status = .error("\(error)")
            statusMessage = "Rotate failed: \(error)"
        }
    }

    public func setArmed(_ value: Bool) {
        let snapshot = LifetimeSnapshotFfi(
            durableArmed: value,
            elevatedCaptureGranted: true,
            hasLinkKey: (try? store.load()) != nil,
            quitOptedOut: armedStore.quitOptedOut,
            requiresElevatedCapture: false
        )
        if value && !lifetimeMayEnterArmed(snapshot: snapshot) {
            armed = false
            armedStore.isArmed = false
            statusMessage = "Cannot Arm — lifetime policy blocked"
            return
        }
        armed = value
        armedStore.isArmed = value
        clipboard.setArmed(value)
        refreshStatusMessage()
    }

    private func restoreFields() {
        nickname = nicknameStore.load() ?? ""
        relayUrl = defaultRelayWsUrl()
        armed = armedStore.isArmed
        guard let credentials = try? store.load() else {
            refreshStatusMessage()
            return
        }
        linkKeyText = linkKeyToBase32(key: credentials.linkKey)
        relayUrl = credentials.relayWsUrl
        refreshStatusMessage()
    }

    private func bootstrapSession() {
        do {
            if let credentials = try store.load() {
                try join(credentials: credentials)
            }
        } catch {
            status = .syncIdle("\(error)")
            statusMessage = "Could not restore session: \(error)"
        }
    }

    private func join(credentials: ShellCredentials) throws {
        clipboard.detach()
        let session = try Session(
            linkKeyBytes: credentials.linkKey,
            relayWsUrl: credentials.relayWsUrl,
            ephemeralIdBytes: credentials.ephemeralId
        )
        session.setArmed(armed: armedStore.isArmed)
        clipboard.attach(session: session)
        armed = armedStore.isArmed
        status = .joined
        refreshStatusMessage()
    }

    private func startStatusPoll() {
        statusTimer?.invalidate()
        statusTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.pollSyncIdle()
            }
        }
        if let statusTimer {
            RunLoop.main.add(statusTimer, forMode: .common)
        }
    }

    private func pollSyncIdle() {
        guard clipboard.hasSession else { return }
        if clipboard.isSyncIdle {
            status = .syncIdle("reconnecting to relay")
            statusMessage = titlePrefix() + " — Sync Idle (reconnecting)"
        } else if case .syncIdle = status {
            status = .joined
            refreshStatusMessage()
        }
    }

    private func refreshStatusMessage() {
        var message = titlePrefix()
        if clipboard.hasSession {
            message += armed ? " — Armed" : " — Paused"
        }
        statusMessage = message
    }

    private func titlePrefix() -> String {
        let nick = nicknameStore.load()
        return nick.map { "Sync Clip · \($0)" } ?? "Sync Clip"
    }

    private func normalizedRelay(_ raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? defaultRelayWsUrl() : trimmed
    }
}
