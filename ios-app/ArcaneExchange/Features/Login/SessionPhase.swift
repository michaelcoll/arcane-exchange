/// What the Clerk session lets the app show: the tabs, the login screen's sign-in card, or
/// neither yet.
enum SessionPhase: Equatable {
    /// Clerk is still restoring the session: nobody can tell yet whether a player is signed in.
    case loading
    case signedOut
    case signedIn

    init(isLoaded: Bool, isSignedIn: Bool) {
        if isSignedIn {
            self = .signedIn
        } else if isLoaded {
            self = .signedOut
        } else {
            self = .loading
        }
    }
}
