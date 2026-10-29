import APIClient
import ClerkKit
import SwiftUI

/// The login screen (`ScrLogin` in the iOS mockup), shown while nobody is signed in: the
/// background, and the sign-in card at the bottom.
struct LoginView: View {
    /// Never `.signedIn`: `ContentView` shows the tabs then.
    let phase: SessionPhase
    /// The public endpoints are read with the anonymous client: a 401 there signs nobody out.
    let api: any APIProtocol

    /// `nil` until the public stats answer, and for good when they do not.
    @State private var proposedCopies: Int?

    var body: some View {
        ZStack(alignment: .bottom) {
            // Only a signed-out visitor fetches the showcase: while the session loads, the
            // wall of the previous launch is enough.
            LoginBackground(api: api, fetchesShowcase: phase == .signedOut)
                .ignoresSafeArea()
            // Hidden until Clerk has restored the session: a signed-in player must not see the
            // card flash before the collection.
            if phase == .signedOut {
                LoginCard(tagline: LoginCopy.tagline(proposedCopies: proposedCopies))
                    .padding(.horizontal, 10)
                    .padding(.bottom, 10)
                    .transition(.opacity)
            }
        }
        .animation(.easeOut(duration: 0.25), value: phase)
        // Only once nobody is signed in: a signed-in player's launch does not fetch stats for a
        // card that never shows.
        .task(id: phase) {
            guard phase == .signedOut else { return }
            proposedCopies = await LoginStats.proposedCopies(api: api)
        }
    }
}

/// What sits behind the sign-in card, filling the whole screen: the app background, and the
/// showcase wall over it — there at once when the previous launch left it on disk, fading in
/// when its images had to be downloaded. An unavailable showcase leaves the background plain,
/// without any message.
struct LoginBackground: View {
    let fetchesShowcase: Bool

    @State private var showcase: ShowcaseModel

    init(api: any APIProtocol, fetchesShowcase: Bool) {
        self.fetchesShowcase = fetchesShowcase
        _showcase = State(initialValue: ShowcaseModel(api: api))
    }

    var body: some View {
        ZStack {
            Color(.systemBackground)
            if !showcase.images.isEmpty {
                ShowcaseWall(images: showcase.images)
                    .transition(.opacity)
            }
        }
        .animation(showcase.fadesIn ? .easeOut(duration: 0.8) : nil, value: showcase.images.isEmpty)
        // Restoring reads the caches only, so it can run while the session is still loading.
        .task(id: fetchesShowcase) {
            await showcase.restore()
            if fetchesShowcase {
                await showcase.refresh()
            }
        }
    }
}

#Preview("Signed out") {
    LoginView(phase: .signedOut, api: APIClientFactory.anonymous())
        .environment(Clerk.shared)
}

#Preview("Session loading") {
    LoginView(phase: .loading, api: APIClientFactory.anonymous())
}
