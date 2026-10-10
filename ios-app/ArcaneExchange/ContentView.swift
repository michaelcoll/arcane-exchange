import ClerkKit
import SwiftUI

struct ContentView: View {
    @Environment(Clerk.self) private var clerk

    private var phase: SessionPhase {
        SessionPhase(isLoaded: clerk.isLoaded, isSignedIn: clerk.user != nil)
    }

    var body: some View {
        Group {
            if phase == .signedIn {
                RootTabView()
            } else {
                LoginView(phase: phase)
            }
        }
        // Keyed on Clerk's two flags rather than on `phase`, which is `.signedIn` as soon as the
        // player is known: the widgets reload once Clerk has finished loading.
        .onChange(of: [clerk.isLoaded, clerk.user != nil], initial: true) {
            SignOutCleanup.run(isLoaded: clerk.isLoaded, isSignedIn: clerk.user != nil)
            // Signing in or out changes whose data the widgets show.
            if clerk.isLoaded {
                HomeWidgetReload.reloadAll()
            }
        }
    }
}
