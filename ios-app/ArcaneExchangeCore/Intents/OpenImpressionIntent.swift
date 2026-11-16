import AppIntents
import Foundation

/// Opens an impression's page in the app, on the Rechercher tab, through its link: the app
/// looks it up again by set and number. It has no call of its own to fail: signed out, the app
/// it opens shows the sign-in, and keeps the impression for after.
package struct OpenImpressionIntent: OpenIntent {
    package static let title: LocalizedStringResource = "Ouvrir une carte"
    package static let description = IntentDescription("Ouvre la fiche d'une carte dans Arcane Exchange.")

    /// What Spotlight launches on a tap on an indexed impression.
    @Parameter(title: "Carte")
    package var target: ImpressionEntity

    package static var parameterSummary: some ParameterSummary {
        Summary("Ouvrir \(\.$target)")
    }

    package init() {}

    package init(target: ImpressionEntity) {
        self.target = target
    }

    package func perform() async throws -> some IntentResult & OpensIntent {
        .result(opensIntent: OpenURLIntent(HomeWidgetLink.impression(target.key).url))
    }
}
