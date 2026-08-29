import ClerkKit
import SwiftUI

@main
struct ArcaneExchangeApp: App {
    init() {
        AppConfig.seedSettingsDefaults()
        ArtworkPipeline.install()
        Clerk.configure(publishableKey: AppConfig.clerkPublishableKey)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(Clerk.shared)
        }
    }
}
