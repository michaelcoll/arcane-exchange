import ClerkKit
import ClerkKitUI
import SwiftUI

struct ContentView: View {
    @Environment(Clerk.self) private var clerk
    @State private var authIsPresented = false

    var body: some View {
        Group {
            if clerk.user != nil {
                RootTabView()
            } else {
                VStack(spacing: 16) {
                    Text("Arcane Exchange")
                    Button("Sign in") {
                        authIsPresented = true
                    }
                }
                .padding()
                .sheet(isPresented: $authIsPresented) {
                    AuthView()
                }
            }
        }
        .onChange(of: [clerk.isLoaded, clerk.user != nil], initial: true) {
            SignOutCleanup.run(isLoaded: clerk.isLoaded, isSignedIn: clerk.user != nil)
            // Signing in or out changes whose data the widgets show.
            if clerk.isLoaded {
                HomeWidgetReload.reloadAll()
            }
        }
    }
}
