import Foundation
import Security

/// Secure Link Key + ephemeral id + relay URL storage in the Keychain.
public struct ShellCredentials: Equatable {
    public var ephemeralId: Data
    public var linkKey: Data
    public var relayWsUrl: String

    public init(ephemeralId: Data, linkKey: Data, relayWsUrl: String) {
        self.ephemeralId = ephemeralId
        self.linkKey = linkKey
        self.relayWsUrl = relayWsUrl
    }
}

public protocol LinkKeyStoring {
    func delete() throws
    func load() throws -> ShellCredentials?
    func save(_ credentials: ShellCredentials) throws
}

public final class KeychainLinkKeyStore: LinkKeyStoring {
    private let service: String

    public init(service: String = "com.syncclip.macos-shell") {
        self.service = service
    }

    public func save(_ credentials: ShellCredentials) throws {
        let payload = try JSONEncoder().encode(StoredPayload(from: credentials))
        try deleteQuietly()
        let query: [String: Any] = [
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
            kSecAttrAccount as String: "credentials",
            kSecAttrService as String: service,
            kSecClass as String: kSecClassGenericPassword,
            kSecValueData as String: payload,
        ]
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }

    public func load() throws -> ShellCredentials? {
        let query: [String: Any] = [
            kSecAttrAccount as String: "credentials",
            kSecAttrService as String: service,
            kSecClass as String: kSecClassGenericPassword,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnData as String: true,
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess, let data = item as? Data else {
            throw KeychainError.loadFailed(status)
        }
        let payload = try JSONDecoder().decode(StoredPayload.self, from: data)
        return payload.credentials()
    }

    public func delete() throws {
        try deleteQuietly()
    }

    private func deleteQuietly() throws {
        let query: [String: Any] = [
            kSecAttrAccount as String: "credentials",
            kSecAttrService as String: service,
            kSecClass as String: kSecClassGenericPassword,
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status)
        }
    }
}

public enum KeychainError: Error {
    case deleteFailed(OSStatus)
    case loadFailed(OSStatus)
    case saveFailed(OSStatus)
}

private struct StoredPayload: Codable {
    var ephemeralIdBase64: String
    var linkKeyBase64: String
    var relayWsUrl: String

    init(from credentials: ShellCredentials) {
        self.ephemeralIdBase64 = credentials.ephemeralId.base64EncodedString()
        self.linkKeyBase64 = credentials.linkKey.base64EncodedString()
        self.relayWsUrl = credentials.relayWsUrl
    }

    func credentials() -> ShellCredentials {
        ShellCredentials(
            ephemeralId: Data(base64Encoded: ephemeralIdBase64) ?? Data(),
            linkKey: Data(base64Encoded: linkKeyBase64) ?? Data(),
            relayWsUrl: relayWsUrl
        )
    }
}
