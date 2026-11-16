import APIClient
import ClerkKit

/// The session and the API client the App Intents run with: the Clerk session the app opened,
/// read from the shared Keychain, as the widgets do. An intent can run before the app has set
/// anything up, so Clerk is configured here on first use.
package enum IntentSession {
    /// The one lookup of the intents: every intent and every entity query goes through it.
    package static let lookup = ImpressionLookup(
        api: APIClientFactory.authenticated(onUnauthorized: {}),
        isSignedIn: { await isSignedIn() }
    )

    @MainActor
    private static func isSignedIn() -> Bool {
        ClerkSetup.configure() && Clerk.shared.session != nil
    }
}
