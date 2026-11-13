import APIClient
import Foundation

/// What a catalog search is for, from the scan batch list.
package enum ScanCorrectionTarget: Hashable, Identifiable, Sendable {
    /// Replace the card of this line, a wrong reading, keeping its quantity, language and finish.
    case replace(lineID: ScanBatchLine.Key)
    /// Add a card the camera does not read (token, card from before 2014, non numeric number).
    case add

    package var id: Self {
        self
    }
}

/// Backs the catalog search opened from the scan batch: the search is the one of the manual
/// addition, and the impression picked either replaces a line's card or joins the batch.
@MainActor
@Observable
package final class ScanCorrectionViewModel {
    package let target: ScanCorrectionTarget
    package let search: CatalogSearchViewModel

    private let scanner: ScannerViewModel
    private let memory: DeviceMemory

    package init(
        target: ScanCorrectionTarget,
        scanner: ScannerViewModel,
        api: any APIProtocol,
        defaults: UserDefaults = .standard
    ) {
        self.target = target
        self.scanner = scanner
        search = CatalogSearchViewModel(api: api)
        memory = DeviceMemory(defaults: defaults)
    }

    /// Applies the impression the player picked. An addition is in the language the manual addition
    /// used last (French at first) and in the finish of the Foil switch.
    package func choose(_ catalogCard: CatalogCard) {
        let card = RecognisedCard(
            set: catalogCard.set.code.uppercased(),
            number: catalogCard.collector_number,
            language: .default,
            rarity: catalogCard.rarity_code,
            name: catalogCard.name,
            frenchName: catalogCard.french_name
        )
        switch target {
        case let .replace(lineID):
            scanner.replaceCard(ofLine: lineID, with: card)
        case .add:
            let language = memory.read(CardLanguage.self, for: PlayerKey.addCardLanguage) ?? .default
            scanner.addFromCatalog(card, language: language)
        }
    }
}
