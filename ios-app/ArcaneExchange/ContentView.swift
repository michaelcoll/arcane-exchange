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
        // Only a signed-in → signed-out transition: the launch goes the other way while Clerk
        // restores the session, and must keep what the previous run remembered.
        .onChange(of: clerk.user == nil) { wasSignedOut, isSignedOut in
            if !wasSignedOut, isSignedOut {
                CollectionFiltersStore().clear()
            }
        }
    }
}
