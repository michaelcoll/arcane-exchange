import ClerkKit
import ClerkKitUI
import SwiftUI

struct ContentView: View {
    @Environment(Clerk.self) private var clerk
    @State private var authIsPresented = false

    var body: some View {
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
}
