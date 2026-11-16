import APIClient

/// Finds the impression a link names, for the app to open it: the same lookup as the App
/// Intents, so a link and Siri land on exactly the same impression.
package struct ImpressionLinkResolver: Sendable {
    private let lookup: ImpressionLookup

    /// The app only follows a link once it shows a signed-in tab, hence no session check.
    package init(api: any APIProtocol) {
        lookup = ImpressionLookup(api: api, isSignedIn: { true })
    }

    /// The impression of a queued link, or nil when it is not found or the call failed: the
    /// tab then stays as it was.
    package func card(for pending: AppRouter.PendingImpression) async -> SearchCard? {
        try? await lookup.impression(for: pending.key)
    }
}
