import AppIntents

/// Answers whether the player owns an impression, without opening the app: the quantity is
/// read from the impression as it is when the question is asked.
package struct OwnedIntent: AppIntent {
    package static let title: LocalizedStringResource = "Posséder une carte"
    package static let description = IntentDescription("Dit si tu possèdes une carte, et en combien d'exemplaires.")
    package static let supportedModes: IntentModes = [.background, .foreground(.dynamic)]

    @Parameter(title: "Carte")
    package var impression: ImpressionEntity

    package static var parameterSummary: some ParameterSummary {
        Summary("Est-ce que j'ai \(\.$impression)")
    }

    package init() {}

    package init(impression: ImpressionEntity) {
        self.impression = impression
    }

    package func perform() async throws -> some IntentResult & ProvidesDialog {
        let outcome = try await lookUp(impression)
        return .result(dialog: IntentDialog(stringLiteral: outcome.text(OwnedCopy.text(of:))))
    }
}
