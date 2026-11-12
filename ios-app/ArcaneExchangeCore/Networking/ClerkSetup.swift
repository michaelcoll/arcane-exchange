import ClerkKit

package enum ClerkSetup {
    /// Configures Clerk the same way in the app and in the widget extension, so both read one
    /// session: same Keychain service, same shared access group. The widget has no sign-in UI
    /// of its own; it only ever refreshes the token of the session the app opened.
    ///
    /// Moving the session into the shared group leaves the one an earlier build stored behind:
    /// after the update the player signs in once more.
    ///
    /// Returns `false` and leaves Clerk unconfigured when the build has no publishable key (a
    /// clone without `Config/Local.xcconfig`): Clerk traps on a blank key in `configure`, and
    /// in Debug on any `Clerk.shared` read before it. The caller must then never touch
    /// `Clerk.shared`.
    @MainActor
    package static func configure() -> Bool {
        let publishableKey = AppConfig.clerkPublishableKey()
        guard !publishableKey.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        Clerk.configure(
            publishableKey: publishableKey,
            options: .init(
                keychainConfig: .init(
                    service: SharedContainer.keychainService,
                    accessGroup: SharedContainer.keychainAccessGroup
                )
            )
        )
        return true
    }
}
