import Foundation

/// Optional Local Nickname stored only on this Device for UI (never on the wire).
public protocol LocalNicknameStoring: AnyObject {
    func clear()
    func load() -> String?
    func save(_ nickname: String)
}

public final class UserDefaultsLocalNicknameStore: LocalNicknameStoring {
    private let defaults: UserDefaults
    private let key: String

    public init(
        defaults: UserDefaults = .standard,
        key: String = "com.syncclip.shell.localNickname"
    ) {
        self.defaults = defaults
        self.key = key
    }

    public func load() -> String? {
        let value = defaults.string(forKey: key)?.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let value, !value.isEmpty else { return nil }
        return value
    }

    public func save(_ nickname: String) {
        let trimmed = nickname.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            clear()
        } else {
            defaults.set(trimmed, forKey: key)
        }
    }

    public func clear() {
        defaults.removeObject(forKey: key)
    }
}
