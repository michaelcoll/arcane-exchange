import APIClient
import ArcaneCore
import ClerkKit
import SwiftUI

/// Launched by `AppEntryPoint`, except when the process only hosts the unit tests.
struct ArcaneExchangeApp: App {
    /// One CoreMotion source for the whole app, read by every `FoilOverlay`.
    @State private var tilt = TiltProvider()
    @State private var router = AppRouter()
    /// The one API client of the session. A 401 signs the player out; `ContentView` follows
    /// the Clerk session back to the login screen.
    private let api: any APIProtocol
    private let avatarStore: PlayerAvatarStore
    /// Lets the next 401 sign the player out again once they have signed back in.
    private let unauthorizedLatch = UnauthorizedLatch()
    /// `false` on a build without a Clerk publishable key: the app then shows
    /// `MissingClerkKeyView` and never reads `Clerk.shared`, which would trap.
    private let hasClerk: Bool

    @Environment(\.scenePhase) private var scenePhase

    init() {
        AppConfig.seedSettingsDefaults()
        AppConfig.shareAPIBaseURL()
        ArtworkPipeline.install()
        hasClerk = ClerkSetup.configure()
        let api = APIClientFactory.authenticated(
            onUnauthorized: { try? await Clerk.shared.auth.signOut() },
            onWidgetsChanged: HomeWidgetReload.reload,
            latch: unauthorizedLatch
        )
        self.api = api
        avatarStore = PlayerAvatarStore(api: api)
    }

    var body: some Scene {
        WindowGroup {
            if hasClerk {
                ContentView()
                    .environment(Clerk.shared)
                    .environment(tilt)
                    .environment(router)
                    .environment(\.api, api)
                    .environment(\.avatarStore, avatarStore)
                    .onChange(of: Clerk.shared.user != nil) { _, isSignedIn in
                        if isSignedIn {
                            Task { await unauthorizedLatch.rearm() }
                        }
                    }
                    .onOpenURL { url in
                        if let link = HomeWidgetLink(url: url) {
                            router.open(link)
                        }
                    }
            } else {
                MissingClerkKeyView()
            }
        }
        .onChange(of: scenePhase) { _, phase in
            switch phase {
            case .active:
                AppConfig.shareAPIBaseURL()
            case .background:
                // Whatever changed during the session, the widgets pick it up as the user
                // leaves — the app is still awake, so this costs no extra wake-up.
                HomeWidgetReload.reloadAll()
            default:
                break
            }
        }
    }
}
