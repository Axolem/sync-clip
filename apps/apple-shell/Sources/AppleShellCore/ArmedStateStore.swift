import Foundation

/// Durable Armed/Paused + Quit auto-start opt-out (ADR-0006).
public protocol ArmedStateStoring: AnyObject {
    var isArmed: Bool { get set }
    var quitOptedOut: Bool { get set }
    func clearQuitOptOut()
}

public final class UserDefaultsArmedStateStore: ArmedStateStoring {
    private let armedKey: String
    private let defaults: UserDefaults
    private let quitKey: String

    public init(
        defaults: UserDefaults = .standard,
        armedKey: String = "com.syncclip.shell.durableArmed",
        quitKey: String = "com.syncclip.shell.quitOptedOut"
    ) {
        self.armedKey = armedKey
        self.defaults = defaults
        self.quitKey = quitKey
    }

    public var isArmed: Bool {
        get {
            if defaults.object(forKey: armedKey) == nil {
                return true
            }
            return defaults.bool(forKey: armedKey)
        }
        set { defaults.set(newValue, forKey: armedKey) }
    }

    public var quitOptedOut: Bool {
        get { defaults.bool(forKey: quitKey) }
        set { defaults.set(newValue, forKey: quitKey) }
    }

    public func clearQuitOptOut() {
        quitOptedOut = false
    }
}
