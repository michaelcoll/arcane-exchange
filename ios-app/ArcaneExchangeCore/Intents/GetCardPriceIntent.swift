import AppIntents
import Foundation

/// Says what an impression is worth, without opening the app: the trend price, normal and foil
/// when it has them.
package struct GetCardPriceIntent: AppIntent {
    package static let title: LocalizedStringResource = "Prix d'une carte"
    package static let description = IntentDescription("Donne le prix de tendance d'une carte.")

    package static let supportedModes: IntentModes = [.background, .foreground(.dynamic)]

    @Parameter(title: "Carte")
    package var impression: ImpressionEntity

    package static var parameterSummary: some ParameterSummary {
        Summary("Prix de \(\.$impression)")
    }

    package init() {}

    package init(impression: ImpressionEntity) {
        self.impression = impression
    }

    package func perform() async throws -> some IntentResult & ProvidesDialog {
        let outcome = try await lookUp(impression)
        return .result(dialog: IntentDialog(stringLiteral: outcome.text(PriceCopy.text(of:))))
    }
}

/// What Siri says about an impression's price.
package enum PriceCopy {
    /// The trend prices, as the app shows them: "Anneau solaire vaut 9,50 €, et 28 € en foil."
    /// A price that does not exist is never said as zero.
    package static func text(of card: SearchCard) -> String {
        let name = card.displayName
        let normal = card.price_guide?.trend.map { Price.euros(cents: Int($0)) }
        let foil = card.foil_price_guide?.trend.map { Price.euros(cents: Int($0)) }
        switch (normal, foil) {
        case let (normal?, foil?): return "\(name) vaut \(normal), et \(foil) en foil."
        case let (normal?, nil): return "\(name) vaut \(normal)."
        case let (nil, foil?): return "\(name) vaut \(foil) en foil."
        case (nil, nil): return "\(name) n'a pas de prix."
        }
    }
}
