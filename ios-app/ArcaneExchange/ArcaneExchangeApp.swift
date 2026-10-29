import APIClient
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

    @Environment(\.scenePhase) private var scenePhase

    init() {
        AppConfig.seedSettingsDefaults()
        AppConfig.shareAPIBaseURL()
        ArtworkPipeline.install()
        ClerkSetup.configure()
        api = APIClientFactory.authenticated(onUnauthorized: { try? await Clerk.shared.auth.signOut() })
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(Clerk.shared)
                .environment(tilt)
                .environment(router)
                .environment(\.api, api)
                .onOpenURL { url in
                    if let link = HomeWidgetLink(url: url) {
                        router.open(link)
                    }
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
