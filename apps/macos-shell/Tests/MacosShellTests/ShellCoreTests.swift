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

final class InMemoryLinkKeyStoreTests: XCTestCase {
    func testRoundTripCredentials() throws {
        let store = InMemoryLinkKeyStore()
        let credentials = ShellCredentials(
            ephemeralId: Data(repeating: 1, count: 16),
            linkKey: Data(repeating: 2, count: 32),
            relayWsUrl: "ws://127.0.0.1:7120/v0/ws"
        )
        try store.save(credentials)
        let loaded = try store.load()
        XCTAssertEqual(loaded, credentials)
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
