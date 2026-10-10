import Testing

@testable import ArcaneExchange

struct SessionPhaseTests {
    /// At launch Clerk has no user until it restores the session: showing the sign-in card
    /// then would make it flash before a signed-in player's collection.
    @Test func hidesTheSignInCardWhileTheSessionIsStillLoading() {
        #expect(SessionPhase(isLoaded: false, isSignedIn: false) == .loading)
    }

    /// Covers a first launch, a sign-out and a session that expired while the app was closed.
    @Test func showsTheSignInCardOnceNobodyIsSignedIn() {
        #expect(SessionPhase(isLoaded: true, isSignedIn: false) == .signedOut)
    }

    @Test func showsTheAppToASignedInPlayer() {
        #expect(SessionPhase(isLoaded: true, isSignedIn: true) == .signedIn)
    }

    /// A user Clerk already knows is enough: the app does not wait for the rest of the load.
    @Test func showsTheAppAsSoonAsThePlayerIsKnown() {
        #expect(SessionPhase(isLoaded: false, isSignedIn: true) == .signedIn)
    }
}
