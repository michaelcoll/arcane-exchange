import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct ScanCorrectionViewModelTests {
    private final class MemoryStorage: ScanBatchStorage {
        var stored: ScanBatch?
        func load() -> ScanBatch? { stored }
        func save(_ batch: ScanBatch) { stored = batch }
    }

    private func catalogCard(_ number: String, set: String = "LEA") -> CatalogCard {
        CatalogCard(
            collector_number: number,
            french_name: "Foudre",
            image_url: nil,
            name: "Lightning Bolt",
            rarity_code: "C",
            set: .init(code: set, name: "Limited Edition Alpha")
        )
    }

    private func scanner(storage: MemoryStorage = MemoryStorage()) -> ScannerViewModel {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let index = ScanIndexModel(api: FakeAPI(), store: ScanIndexStore(file: directory.appendingPathComponent("i.json")))
        return ScannerViewModel(index: index, storage: storage)
    }

    /// A scanner whose batch holds the line of the card MOM #1 in German, foil, three copies.
    private func scannerWithALine() -> ScannerViewModel {
        let scanner = scanner()
        scanner.foil = true
        scanner.addFromCatalog(
            RecognisedCard(set: "MOM", number: "1", language: .german, rarity: "R", name: "Wrong", frenchName: nil),
            language: .german
        )
        scanner.setQuantity(3, ofLine: scanner.batch.lines[0].id)
        return scanner
    }

    private func searchApi(returning cards: [CatalogCard]) -> FakeAPI {
        let api = FakeAPI()
        api.onSearchCatalog = { _ in
            .ok(.init(body: .json(.init(items: cards, page: 0, page_size: 20, total: Int64(cards.count)))))
        }
        return api
    }

    @Test func replacingALineWithAChosenImpressionKeepsQuantityLanguageAndFinish() async {
        let scanner = scannerWithALine()
        let model = ScanCorrectionViewModel(
            target: .replace(lineID: scanner.batch.lines[0].id),
            scanner: scanner,
            api: searchApi(returning: [catalogCard("161")]),
            defaults: isolatedDefaults()
        )

        model.search.query = "foudre"
        await model.search.paginator.reload()
        model.choose(model.search.paginator.items[0])

        #expect(scanner.batch.lines.count == 1)
        let line = scanner.batch.lines[0]
        #expect(line.set == "LEA")
        #expect(line.number == "161")
        #expect(line.displayName == "Foudre")
        #expect(line.rarity == "C")
        #expect(line.quantity == 3)
        #expect(line.language == .german)
        #expect(line.foil)
    }

    @Test func addingAChosenImpressionTakesTheSearchDefaultLanguageAndTheFoilSwitch() {
        let defaults = isolatedDefaults()
        DeviceMemory(defaults: defaults).write(CardLanguage.japanese, for: PlayerKey.addCardLanguage)
        let scanner = scanner()
        scanner.foil = true
        let model = ScanCorrectionViewModel(target: .add, scanner: scanner, api: FakeAPI(), defaults: defaults)

        model.choose(catalogCard("161"))

        #expect(scanner.batch.lines.count == 1)
        let line = scanner.batch.lines[0]
        #expect(line.quantity == 1)
        #expect(line.language == .japanese)
        #expect(line.foil)
        #expect(line.number == "161")
    }

    @Test func addingFallsBackOnTheDefaultLanguageWhenNoneWasChosenBefore() {
        let scanner = scanner()
        let model = ScanCorrectionViewModel(target: .add, scanner: scanner, api: FakeAPI(), defaults: isolatedDefaults())

        model.choose(catalogCard("161"))

        #expect(scanner.batch.lines[0].language == .default)
        #expect(!scanner.batch.lines[0].foil)
    }

    @Test func aCorrectionIsSavedWithTheBatchAndWithdrawsTheUndoOffer() {
        let storage = MemoryStorage()
        let scanner = scanner(storage: storage)
        scanner.addFromCatalog(
            RecognisedCard(set: "MOM", number: "1", language: .french, rarity: "R", name: "X", frenchName: nil),
            language: .french
        )

        scanner.setLanguage(.english, ofLine: scanner.batch.lines[0].id)
        scanner.setFoil(true, ofLine: scanner.batch.lines[0].id)

        #expect(storage.stored == scanner.batch)
        #expect(storage.stored?.lines[0].language == .english)
        #expect(storage.stored?.lines[0].foil == true)
        #expect(scanner.undoOffer == nil)
    }

    @Test func aLanguageCorrectionMergesTwoLinesMadeIdentical() {
        let scanner = scanner()
        let card = RecognisedCard(set: "MOM", number: "1", language: .french, rarity: "R", name: "X", frenchName: nil)
        scanner.addFromCatalog(card, language: .french)
        scanner.addFromCatalog(card, language: .english)

        scanner.setLanguage(.french, ofLine: scanner.batch.lines[0].id)

        #expect(scanner.batch.lines.count == 1)
        #expect(scanner.batch.lines[0].quantity == 2)
    }
}
