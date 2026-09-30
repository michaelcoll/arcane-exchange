import Foundation

/// What the device forgets once nobody is signed in — the one place to list data that belongs to
/// the previous player.
///
/// "Signed out" waits for Clerk to have loaded: at launch it has no user until the session is
/// restored, and that must not wipe what the previous run remembered. Checking the state rather
/// than a sign-out transition also covers a session that expired while the app was closed.
enum SignOutCleanup {
    static func run(isLoaded: Bool, isSignedIn: Bool, defaults: UserDefaults = .standard) {
        guard isLoaded, !isSignedIn else { return }
        CollectionFiltersStore(defaults: defaults).clear()
    }
}
