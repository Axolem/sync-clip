import AppleShellCore
import XCTest

final class PasteboardEchoGuardTests: XCTestCase {
    func testRemoteWriteIsIgnoredOnce() {
        let guard_ = PasteboardEchoGuard()
        guard_.markRemoteWrite(text: "remote clip")
        XCTAssertTrue(guard_.shouldIgnoreChange(currentText: "remote clip"))
        XCTAssertFalse(guard_.shouldIgnoreChange(currentText: "user copy"))
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
        XCTAssertEqual(try store.load(), credentials)
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
}

final class ClipboardSyncControllerTests: XCTestCase {
    func testPublishesLocalTextWhenArmed() throws {
        let clipboard = FakeClipboard()
        let session = FakeSession()
        session.armed = true
        let controller = ClipboardSyncController(clipboard: clipboard)
        controller.attach(session: session)
        controller.stopLoops()

        clipboard.changeCount = 2
        clipboard.snapshot = LocalClipboardSnapshot(text: "hello from device")
        controller.pollLocalClipboard()

        XCTAssertEqual(session.publishedTexts, ["hello from device"])
    }

    func testIgnoresEchoAfterRemoteApply() {
        let clipboard = FakeClipboard()
        let session = FakeSession()
        session.armed = true
        session.nextApplied = AppliedClipFfi(
            createdAt: 1,
            idHex: "ab",
            imageBytes: nil,
            imageMime: nil,
            text: "from remote device"
        )
        let controller = ClipboardSyncController(clipboard: clipboard)
        controller.attach(session: session)
        controller.stopLoops()

        controller.pollRemoteApplied()
        XCTAssertEqual(clipboard.writtenTexts, ["from remote device"])

        clipboard.changeCount += 1
        clipboard.snapshot = LocalClipboardSnapshot(text: "from remote device")
        controller.pollLocalClipboard()
        XCTAssertTrue(session.publishedTexts.isEmpty)
    }

    func testDoesNotPublishWhenPaused() {
        let clipboard = FakeClipboard()
        let session = FakeSession()
        session.armed = false
        let controller = ClipboardSyncController(clipboard: clipboard)
        controller.attach(session: session)
        controller.stopLoops()

        clipboard.changeCount = 3
        clipboard.snapshot = LocalClipboardSnapshot(text: "paused copy")
        controller.pollLocalClipboard()
        XCTAssertTrue(session.publishedTexts.isEmpty)
    }
}

final class FakeClipboard: SystemClipboard {
    var changeCount: Int = 1
    var snapshot = LocalClipboardSnapshot()
    var writtenTexts: [String] = []

    func readSnapshot() -> LocalClipboardSnapshot {
        snapshot
    }

    func writeApplied(_ applied: AppliedClipFfi) {
        writtenTexts.append(applied.text)
        snapshot = LocalClipboardSnapshot(text: applied.text)
        changeCount += 1
    }
}

final class FakeSession: ClipSessioning {
    var armed = false
    var syncIdle = false
    var nextApplied: AppliedClipFfi?
    var publishedTexts: [String] = []

    func isArmed() -> Bool { armed }
    func isSyncIdle() -> Bool { syncIdle }
    func setArmed(armed: Bool) { self.armed = armed }

    func publishText(text: String) throws {
        publishedTexts.append(text)
    }

    func publishTextAndImage(text: String, imageBytes: Data, imageMime: String) throws {
        publishedTexts.append(text)
    }

    func pollApplied() -> AppliedClipFfi? {
        let value = nextApplied
        nextApplied = nil
        return value
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
