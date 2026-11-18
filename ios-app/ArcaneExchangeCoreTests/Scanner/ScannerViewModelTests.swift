import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct ScannerViewModelTests {
    let lines = ["Elesh Norn", "R 0123", "MOM • FR", "Jean Dupont"]

    private func index(loaded: Bool) async -> ScanIndexModel {
        let api = FakeAPI()
        api.onGetScanIndex = { _ in
            loaded
                ? .ok(.init(headers: .init(ETag: "\"v1\""), body: .json([
                    .init(collector_number: "123", french_name: "Elesh Norn, Mère des Machines",
                          name: "Elesh Norn", rarity: "R", set_code: "MOM"),
                    .init(collector_number: "45", french_name: nil, name: "Reprieve", rarity: "C",
                          set_code: "MOM"),
                ])))
                : .notModified(.init())
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let model = ScanIndexModel(api: api, store: ScanIndexStore(file: directory.appendingPathComponent("i.json")))
        await model.refresh()
        return model
    }

    @Test func showsTheCardOnceItIsSeenOnTwoConsecutiveFrames() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        scanner.frame(lines: lines, at: 0)
        #expect(scanner.lastRecognised == nil)
        scanner.frame(lines: lines, at: 0.2)

        #expect(scanner.lastRecognised?.displayName == "Elesh Norn, Mère des Machines")
        #expect(scanner.lastRecognised?.set == "MOM")
        #expect(scanner.lastRecognised?.number == "123")
        #expect(scanner.recognitionCount == 1)
    }

    @Test func countsTheCardOnceWhileItStaysInFrontOfTheCamera() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        for step in 0..<5 {
            scanner.frame(lines: lines, at: Double(step) * 0.2)
        }

        #expect(scanner.recognitionCount == 1)
    }

    @Test func recognisesNothingWithoutAnIndex() async {
        let scanner = ScannerViewModel(index: await index(loaded: false), storage: MemoryStorage())

        scanner.frame(lines: lines, at: 0)
        scanner.frame(lines: lines, at: 0.2)

        #expect(scanner.lastRecognised == nil)
        #expect(scanner.recognitionCount == 0)
    }

    // MARK: Scan batch

    let otherLines = ["Reprieve", "C 0045", "MOM • FR", "Jean Dupont"]

    private final class MemoryStorage: ScanBatchStorage {
        var stored: ScanBatch?
        func load() -> ScanBatch? { stored }
        func save(_ batch: ScanBatch) { stored = batch }
    }

    /// Shows a card to the camera for three frames, as a player holding it would.
    private func show(_ lines: [String], to scanner: ScannerViewModel, from start: TimeInterval) {
        for step in 0..<3 {
            scanner.frame(lines: lines, at: start + Double(step) * 0.2)
        }
    }

    /// Shows a card, then takes it away, so the same card can be read again.
    private func scan(_ lines: [String], to scanner: ScannerViewModel, from start: TimeInterval) {
        show(lines, to: scanner, from: start)
        showNothing(to: scanner, from: start + 1)
    }

    private func priced(_ api: FakeAPI) async -> ScannerViewModel {
        ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage(), prices: ScanBatchPrices(api: api))
    }

    /// An empty frame for longer than the stabiliser's gap: the card left the camera.
    private func showNothing(to scanner: ScannerViewModel, from start: TimeInterval) {
        scanner.frame(lines: [], at: start)
        scanner.frame(lines: [], at: start + 2)
    }

    @Test func addsARetainedCardToTheBatch() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        show(lines, to: scanner, from: 0)

        #expect(scanner.batch.lines.map(\.number) == ["123"])
        #expect(scanner.batch.lines[0].language == .french)
        #expect(scanner.recognitionCount == 1)
    }

    @Test func countsTheSameCardOnlyOnceEvenAfterItLeftTheCamera() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        show(lines, to: scanner, from: 0)
        showNothing(to: scanner, from: 5)
        show(lines, to: scanner, from: 10)

        #expect(scanner.batch.cardCount == 1)
        #expect(scanner.recognitionCount == 1)
    }

    @Test func countsTheSameCardAgainAfterAnotherOne() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        show(lines, to: scanner, from: 0)
        show(otherLines, to: scanner, from: 5)
        show(lines, to: scanner, from: 10)

        #expect(scanner.batch.cardCount == 3)
        #expect(scanner.recognitionCount == 3)
    }

    @Test func recordsTheCardsInFoilWhileTheSwitchIsOn() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        show(lines, to: scanner, from: 0)
        scanner.foil = true
        show(otherLines, to: scanner, from: 5)

        #expect(scanner.batch.lines.map(\.foil) == [true, false])
    }

    @Test func remembersTheFinishTheLastCardWasReadIn() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        scanner.foil = true
        show(lines, to: scanner, from: 0)

        scanner.foil = false

        #expect(scanner.lastRecognisedFoil)
    }

    @Test func keepsTheFinishOfTheLineWhenTheSameCardIsReadAgain() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        show(lines, to: scanner, from: 0)
        showNothing(to: scanner, from: 5)

        scanner.foil = true
        show(lines, to: scanner, from: 10)

        #expect(scanner.batch.lines.map(\.foil) == [false])
        #expect(!scanner.lastRecognisedFoil)
    }

    @Test func offersToUndoTheCardJustAdded() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        show(lines, to: scanner, from: 0)
        #expect(scanner.undoOffer?.number == "123")

        scanner.undoLastAdd()

        #expect(scanner.batch.lines.isEmpty)
        #expect(scanner.undoOffer == nil)
        // Shown again, the same card counts.
        showNothing(to: scanner, from: 5)
        show(lines, to: scanner, from: 10)
        #expect(scanner.batch.cardCount == 1)
    }

    @Test func withdrawsTheUndoOffer() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        show(lines, to: scanner, from: 0)

        scanner.dismissUndoOffer()
        scanner.undoLastAdd()

        #expect(scanner.undoOffer == nil)
        #expect(scanner.batch.cardCount == 1)
    }

    @Test func withdrawsTheUndoOfferWhenALineIsCorrected() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        show(lines, to: scanner, from: 0)

        scanner.setQuantity(2, ofLine: scanner.batch.lines[0].id)

        #expect(scanner.undoOffer == nil)
        #expect(scanner.batch.cardCount == 2)
    }

    @Test func addsNothingWhilePaused() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        scanner.isPaused = true
        show(lines, to: scanner, from: 0)
        #expect(scanner.batch.lines.isEmpty)
        #expect(scanner.lastRecognised == nil)

        scanner.isPaused = false
        show(lines, to: scanner, from: 5)
        #expect(scanner.batch.cardCount == 1)
    }

    @Test func clearsTheBatch() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())
        show(lines, to: scanner, from: 0)

        scanner.clear()

        #expect(scanner.batch.lines.isEmpty)
        #expect(scanner.undoOffer == nil)
    }

    @Test func savesTheBatchAtEveryChangeAndReadsItBackAtLaunch() async {
        let storage = MemoryStorage()
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: storage)

        show(lines, to: scanner, from: 0)
        show(otherLines, to: scanner, from: 5)
        scanner.setQuantity(3, ofLine: scanner.batch.lines[0].id)

        let relaunched = ScannerViewModel(index: await index(loaded: true), storage: storage)
        #expect(relaunched.batch == scanner.batch)
        #expect(relaunched.batch.id == scanner.batch.id)
        #expect(relaunched.batch.cardCount == 4)
    }

    @Test func startsAnEmptyBatchWhenNothingWasSaved() async {
        let scanner = ScannerViewModel(index: await index(loaded: true), storage: MemoryStorage())

        #expect(scanner.batch.lines.isEmpty)
    }

    @Test func theLastRecognisedPriceIsUnitAndFollowsItsFinish() async {
        let scanner = await priced(FakeAPI.trendPrices(["123": (1000, 5000), "45": (20, nil)]))
        scan(lines, to: scanner, from: 0)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == 1000)

        scan(otherLines, to: scanner, from: 5)
        scanner.foil = true
        scan(lines, to: scanner, from: 10)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == 5000)

        // The third copy of the same card: still the price of one.
        scanner.foil = false
        scan(otherLines, to: scanner, from: 15)
        scan(lines, to: scanner, from: 20)
        scan(otherLines, to: scanner, from: 25)
        scan(lines, to: scanner, from: 30)
        await scanner.refreshPrices()
        #expect(scanner.batch.lines.first { $0.number == "123" && !$0.foil }?.quantity == 3)
        #expect(scanner.lastRecognisedUnitCents == 1000)
    }

    @Test func theLastRecognisedPriceIsNilBeforeTheAnswerWhenUnknownOrHidden() async {
        let api = FakeAPI.trendPrices(["123": (1000, nil)])
        let scanner = await priced(api)
        #expect(scanner.lastRecognisedUnitCents == nil)

        scan(lines, to: scanner, from: 0)
        #expect(scanner.lastRecognisedUnitCents == nil)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == 1000)

        // No foil price for this card.
        scanner.foil = true
        scan(otherLines, to: scanner, from: 5)
        scan(lines, to: scanner, from: 10)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == nil)

        // No price at all for this card.
        scan(otherLines, to: scanner, from: 15)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == nil)

        // Without the network.
        api.onGetTrendPrices = { _ in throw URLError(.notConnectedToInternet) }
        scanner.foil = false
        scan(lines, to: scanner, from: 20)
        await scanner.refreshPrices()
        #expect(scanner.lastRecognisedUnitCents == nil)
    }
}

struct DeviceScanBatchStorageTests {
    @Test func keepsTheBatchInTheDeviceMemory() throws {
        let suite = "scan-batch-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let storage = DeviceScanBatchStorage(defaults: defaults)
        #expect(storage.load() == nil)

        var batch = ScanBatch()
        batch.addFromCamera(
            RecognisedCard(set: "MOM", number: "1", language: .french, rarity: "C", name: "A", frenchName: nil),
            foil: true
        )
        storage.save(batch)

        #expect(DeviceScanBatchStorage(defaults: defaults).load() == batch)
    }
}
