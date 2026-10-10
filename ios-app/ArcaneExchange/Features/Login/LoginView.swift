import ClerkKit
import SwiftUI

/// The login screen (`ScrLogin` in the iOS mockup), shown while nobody is signed in: the
/// background, and the sign-in card at the bottom.
struct LoginView: View {
    /// Never `.signedIn`: `ContentView` shows the tabs then.
    let phase: SessionPhase

    var body: some View {
        ZStack(alignment: .bottom) {
            LoginBackground()
                .ignoresSafeArea()
            // Hidden until Clerk has restored the session: a signed-in player must not see the
            // card flash before the collection.
            if phase == .signedOut {
                LoginCard(tagline: LoginCopy.tagline)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 10)
                    .transition(.opacity)
            }
        }
        .animation(.easeOut(duration: 0.25), value: phase)
    }
}

/// What sits behind the sign-in card, filling the whole screen. Plain for now.
struct LoginBackground: View {
    var body: some View {
        Color(.systemBackground)
    }
}

/// The login screen's own strings. Everything inside the Clerk auth view is Clerk's.
enum LoginCopy {
    static let tagline = "Des cartes vous attendent dans les classeurs des joueurs."
}

#Preview("Signed out") {
    LoginView(phase: .signedOut)
        .environment(Clerk.shared)
}

#Preview("Session loading") {
    LoginView(phase: .loading)
}
