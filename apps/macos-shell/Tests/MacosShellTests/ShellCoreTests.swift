import MacosShellCore
import XCTest

final class PasteboardEchoGuardTests: XCTestCase {
    func testRemoteWriteIsIgnoredOnce() {
        let guard_ = PasteboardEchoGuard()
        guard_.markRemoteWrite(text: "remote clip")
        XCTAssertTrue(guard_.shouldIgnoreChange(currentText: "remote clip"))
        XCTAssertFalse(guard_.shouldIgnoreChange(currentText: "user copy"))
    }

    func testMatchingAppliedTextStillSuppressedWithoutCounter() {
        let guard_ = PasteboardEchoGuard()
        guard_.markRemoteWrite(text: "same")
        _ = guard_.shouldIgnoreChange(currentText: "same")
        // Counter consumed; text match still suppresses a stray duplicate observation.
        XCTAssertTrue(guard_.shouldIgnoreChange(currentText: "same"))
    }
}

final class ArmedStateStoreTests: XCTestCase {
    func testArmedAndQuitOptOutRoundTrip() {
        let suite = "com.syncclip.test.armed.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = UserDefaultsArmedStateStore(
            defaults: defaults,
            armedKey: "armed",
            quitKey: "quit"
        )
        XCTAssertTrue(store.isArmed)
        store.isArmed = false
        XCTAssertFalse(store.isArmed)
        store.quitOptedOut = true
        XCTAssertTrue(store.quitOptedOut)
        store.clearQuitOptOut()
        XCTAssertFalse(store.quitOptedOut)
    }

    func testLifetimePolicyAutoStartViaFfi() {
        let snap = LifetimeSnapshotFfi(
            durableArmed: true,
            elevatedCaptureGranted: true,
            hasLinkKey: true,
            quitOptedOut: false,
            requiresElevatedCapture: false
        )
        XCTAssertTrue(lifetimeMayAutoStart(snapshot: snap))
        XCTAssertFalse(
            lifetimeMayAutoStart(
                snapshot: LifetimeSnapshotFfi(
                    durableArmed: true,
                    elevatedCaptureGranted: true,
                    hasLinkKey: true,
                    quitOptedOut: true,
                    requiresElevatedCapture: false
                )
            )
        )
    }
}

final class InMemoryLinkKeyStoreTests: XCTestCase {
    func testRoundTripCredentials() throws {
        let store = InMemoryLinkKeyStore()
        let credentials = ShellCredentials(
            ephemeralId: Data(repeating: 1, count: 16),
            linkKey: Data(repeating: 2, count: 32),
            relayWsUrl: defaultRelayWsUrl()
        )
        try store.save(credentials)
        let loaded = try store.load()
        XCTAssertEqual(loaded, credentials)
    }

    func testRelayUrlPersistsAcrossSaveLoad() throws {
        let store = InMemoryLinkKeyStore()
        let custom = "ws://127.0.0.1:9999/v0/ws"
        try store.save(
            ShellCredentials(
                ephemeralId: Data(repeating: 3, count: 16),
                linkKey: Data(repeating: 4, count: 32),
                relayWsUrl: custom
            )
        )
        XCTAssertEqual(try store.load()?.relayWsUrl, custom)
        // Simulate relaunch: same store instance with prior save.
        XCTAssertEqual(try store.load()?.relayWsUrl, custom)
    }

    func testRotateClearsOldKeyThenSavesNew() throws {
        let store = InMemoryLinkKeyStore()
        let oldKey = Data(repeating: 9, count: 32)
        try store.save(
            ShellCredentials(
                ephemeralId: Data(repeating: 1, count: 16),
                linkKey: oldKey,
                relayWsUrl: defaultRelayWsUrl()
            )
        )
        try store.delete()
        XCTAssertNil(try store.load())
        let newKey = Data(repeating: 8, count: 32)
        try store.save(
            ShellCredentials(
                ephemeralId: Data(repeating: 1, count: 16),
                linkKey: newKey,
                relayWsUrl: defaultRelayWsUrl()
            )
        )
        XCTAssertEqual(try store.load()?.linkKey, newKey)
        XCTAssertNotEqual(try store.load()?.linkKey, oldKey)
    }
}

final class LocalNicknameStoreTests: XCTestCase {
    func testSetClearPersistLocally() {
        let suite = "com.syncclip.test.nickname.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = UserDefaultsLocalNicknameStore(defaults: defaults)
        XCTAssertNil(store.load())
        store.save("Desk Mac")
        XCTAssertEqual(store.load(), "Desk Mac")
        store.clear()
        XCTAssertNil(store.load())
    }

    func testEmptySaveClearsNickname() {
        let suite = "com.syncclip.test.nickname.empty.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = UserDefaultsLocalNicknameStore(defaults: defaults)
        store.save("temp")
        store.save("   ")
        XCTAssertNil(store.load())
    }
}

final class InMemoryLinkKeyStore: LinkKeyStoring {
    private var value: ShellCredentials?

    func save(_ credentials: ShellCredentials) throws {
        value = credentials
    }

    func load() throws -> ShellCredentials? {
        value
    }

    func delete() throws {
        value = nil
    }
}
