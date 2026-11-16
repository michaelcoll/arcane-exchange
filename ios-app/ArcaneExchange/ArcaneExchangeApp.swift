import APIClient
import ArcaneExchangeCore
import ClerkKit
import SwiftUI

@main
struct ArcaneExchangeApp: App {
    /// One CoreMotion source for the whole app, read by every `FoilOverlay`.
    @State private var tilt = TiltProvider()
    @State private var router = AppRouter()
    /// The one API client of the session. A 401 signs the player out; `ContentView` follows
    /// the Clerk session back to the login screen.
    private let api: any APIProtocol
    private let avatarStore: PlayerAvatarStore
    private let scanIndex: ScanIndexModel
    private let spotlightIndexer: SpotlightIndexer
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
        scanIndex = ScanIndexModel(api: api)
        // Its own client, on the same session: a 401 of the background indexing must not sign the player out.
        spotlightIndexer = SpotlightIndexer(
            api: APIClientFactory.authenticated(onUnauthorized: {}),
            index: CoreSpotlightImpressionIndex(),
            thumbnails: FileThumbnailStore()
        )
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
                    .environment(\.scanIndex, scanIndex)
                    .onChange(of: Clerk.shared.user != nil) { _, isSignedIn in
                        if isSignedIn {
                            Task { await unauthorizedLatch.rearm() }
                        }
                    }
                    // Keyed on Clerk's two flags: at launch it has no user until it restores the
                    // session, which is not a sign-out. Signed out, the next player on the phone
                    // must not find this Collection in Spotlight.
                    .onChange(of: [Clerk.shared.isLoaded, Clerk.shared.user != nil], initial: true) {
                        reconcileOrPurgeSpotlight()
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
                if hasClerk {
                    reconcileOrPurgeSpotlight()
                }
            case .background:
                // Whatever changed during the session, the widgets pick it up as the user
                // leaves — the app is still awake, so this costs no extra wake-up.
                HomeWidgetReload.reloadAll()
            default:
                break
            }
        }
    }

    /// At utility priority: indexing a large Collection must not compete with the interface. The
    /// indexer is an actor, so its work never runs on the main one.
    private func reconcileOrPurgeSpotlight() {
        guard Clerk.shared.isLoaded else { return }
        let indexer = spotlightIndexer
        let isSignedIn = Clerk.shared.user != nil
        Task(priority: .utility) {
            if isSignedIn {
                await indexer.reconcileCollection()
            } else {
                await indexer.purge()
            }
        }
    }
}
