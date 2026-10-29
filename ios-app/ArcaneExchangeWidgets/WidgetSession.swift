import APIClient
import ClerkKit
import Foundation

/// The Clerk session the app opened, read from the shared Keychain.
@MainActor
enum WidgetSession {
    private static var isConfigured = false

    /// Configures Clerk on first use — it restores the cached session synchronously — and
    /// reports whether there is one. The token itself is refreshed by `ClerkAuthMiddleware`
    /// on each request, as in the app.
    static func isSignedIn() -> Bool {
        if !isConfigured {
            ClerkSetup.configure()
            isConfigured = true
        }
        return Clerk.shared.session != nil
    }
}

enum WidgetAPI {
    /// The authenticated client of the widgets. A 401 calls nothing: the widgets share the
    /// app's session and never sign anyone out, they just show their signed-out state.
    static let client = APIClientFactory.authenticated(onUnauthorized: {})
}
