import AppIntents
import ArcaneExchangeCore

/// Declares the App Intents of `ArcaneExchangeCore` to the system: they live in the framework,
/// the app is the one bundle the system reads.
struct ArcaneExchangeAppIntents: AppIntentsPackage {
    static var includedPackages: [any AppIntentsPackage.Type] {
        [ArcaneExchangeCoreIntents.self]
    }
}

/// The Siri phrases, one per intent: each says the app's name, the system requires it. Spoken
/// and shown in French, the app being used in France.
struct ArcaneExchangeShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: OpenImpressionIntent(),
            phrases: ["Ouvre \(\.$impression) dans \(.applicationName)"],
            shortTitle: "Ouvrir une carte",
            systemImageName: "magnifyingglass"
        )
    }
}
