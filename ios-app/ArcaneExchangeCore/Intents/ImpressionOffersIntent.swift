import AppIntents
import Foundation

/// Says how many players offer an impression and, when somebody does, offers to open its offers
/// in the app. Only the Owner count comes out: nothing else of another player. It never opens
/// the app for nobody, nor on a failure.
package struct ImpressionOffersIntent: AppIntent {
    package static let title: LocalizedStringResource = "Qui propose une carte"
    package static let description = IntentDescription(
        "Dit combien de joueurs proposent une carte et propose d'ouvrir ses offres dans Arcane Exchange."
    )
    package static let supportedModes: IntentModes = [.background, .foreground(.dynamic)]

    @Parameter(title: "Carte")
    package var impression: ImpressionEntity

    package static var parameterSummary: some ParameterSummary {
        Summary("Qui propose \(\.$impression)")
    }

    package init() {}

    package init(impression: ImpressionEntity) {
        self.impression = impression
    }

    package func perform() async throws -> some IntentResult & ProvidesDialog & OpensIntent {
        let outcome = try await lookUp(impression)
        let answer = IntentDialog(stringLiteral: outcome.map { Int($0.owner_count) }.text(OffersCopy.text(ownerCount:)))
        guard case let .success(card) = outcome, card.owner_count > 0 else {
            return .result(dialog: answer)
        }
        try await requestConfirmation(
            actionName: .go,
            dialog: IntentDialog(stringLiteral: OffersCopy.prompt(ownerCount: Int(card.owner_count)))
        )
        return .result(
            opensIntent: OpenURLIntent(HomeWidgetLink.impressionOffers(impression.key).url),
            dialog: answer
        )
    }
}

/// What Siri says about the offers of an impression.
package enum OffersCopy {
    package static func text(ownerCount: Int) -> String {
        switch ownerCount {
        case 0: "Personne ne la propose."
        case 1: "1 joueur la propose."
        default: "\(ownerCount) joueurs la proposent."
        }
    }

    /// The answer, then the question of opening the offers.
    package static func prompt(ownerCount: Int) -> String {
        "\(text(ownerCount: ownerCount)) Ouvrir les offres ?"
    }
}
