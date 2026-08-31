import ClerkKit
import SwiftUI

@main
struct ArcaneExchangeApp: App {
    /// One CoreMotion source for the whole app, read by every `FoilOverlay`.
    @State private var tilt = TiltProvider()

    init() {
        AppConfig.seedSettingsDefaults()
        ArtworkPipeline.install()
        Clerk.configure(publishableKey: AppConfig.clerkPublishableKey)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(Clerk.shared)
                .environment(tilt)
        }
    }
}
