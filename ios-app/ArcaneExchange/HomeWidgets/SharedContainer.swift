import Foundation

/// What the app and its widget extension share on the device. Both processes compile this
/// file; the identifiers must match the entitlements declared in `project.yml`.
enum SharedContainer {
    /// The App Group holding the API base URL and the widgets' last snapshots.
    static let appGroup = "group.fr.piconsoft.arcane-exchange"

    /// The App Group's `UserDefaults`. Falls back to the process's own defaults when the group
    /// is not provisioned (a build signed without a team): nothing is shared then, but nothing
    /// crashes either.
    static var defaults: UserDefaults {
        UserDefaults(suiteName: appGroup) ?? .standard
    }

    /// The Keychain service Clerk files its session under. Pinned to the app's bundle id:
    /// ClerkKit defaults to `Bundle.main.bundleIdentifier`, which differs in the extension.
    static let keychainService = "fr.piconsoft.arcane-exchange"

    /// The Keychain group the app and the widget both read the Clerk session from, `nil` on a
    /// build signed without a team (CI): the team prefix is then empty and there is nothing to
    /// share.
    ///
    /// The prefix comes from Info.plist (`AppIdentifierPrefix`, set from the build setting of
    /// the same name): the entitlement names the group `$(AppIdentifierPrefix)…`, and the code
    /// must pass it in full.
    static var keychainAccessGroup: String? {
        guard let prefix = Bundle.main.object(forInfoDictionaryKey: "AppIdentifierPrefix") as? String,
              !prefix.isEmpty
        else {
            return nil
        }
        return prefix + "fr.piconsoft.arcane-exchange.shared"
    }
}
