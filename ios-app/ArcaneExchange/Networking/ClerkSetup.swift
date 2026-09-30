import ClerkKit

enum ClerkSetup {
    /// Configures Clerk the same way in the app and in the widget extension, so both read one
    /// session: same Keychain service, same shared access group. The widget has no sign-in UI
    /// of its own; it only ever refreshes the token of the session the app opened.
    ///
    /// Moving the session into the shared group leaves the one an earlier build stored behind:
    /// after the update the player signs in once more.
    @MainActor
    static func configure() {
        Clerk.configure(
            publishableKey: AppConfig.clerkPublishableKey,
            options: .init(
                keychainConfig: .init(
                    service: SharedContainer.keychainService,
                    accessGroup: SharedContainer.keychainAccessGroup
                )
            )
        )
    }
}
