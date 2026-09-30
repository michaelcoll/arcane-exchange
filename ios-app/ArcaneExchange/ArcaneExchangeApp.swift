import ClerkKit
import SwiftUI

@main
struct ArcaneExchangeApp: App {
    /// One CoreMotion source for the whole app, read by every `FoilOverlay`.
    @State private var tilt = TiltProvider()
    @State private var router = AppRouter()
    @Environment(\.scenePhase) private var scenePhase

    init() {
        AppConfig.seedSettingsDefaults()
        AppConfig.shareAPIBaseURL()
        ArtworkPipeline.install()
        ClerkSetup.configure()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(Clerk.shared)
                .environment(tilt)
                .environment(router)
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
