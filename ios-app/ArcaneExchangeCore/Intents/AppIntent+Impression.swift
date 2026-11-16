import AppIntents

extension AppIntent {
    /// The impression as it is now, or why there is none. Signed out, the player is first
    /// offered to open the app, the one place they can sign in; the failure is returned all the
    /// same, for the intent to word it.
    func lookUp(_ impression: ImpressionEntity) async throws -> Result<SearchCard, IntentFailure> {
        let outcome = await IntentSession.lookup.card(for: impression.key)
        if case .failure(.signedOut) = outcome {
            try await continueInForeground(IntentDialog(stringLiteral: IntentFailure.signedOut.message))
        }
        return outcome
    }
}
