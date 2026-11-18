import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct ScanBatchPricesTests {
    let elesh = ["Elesh Norn", "R 0123", "MOM • FR", "Jean Dupont"]
    let reprieve = ["Reprieve", "C 0045", "MOM • FR", "Jean Dupont"]

    private final class MemoryStorage: ScanBatchStorage {
        var stored: ScanBatch?
        func load() -> ScanBatch? { stored }
        func save(_ batch: ScanBatch) { stored = batch }
    }

    private func index() async -> ScanIndexModel {
        let api = FakeAPI()
        api.onGetScanIndex = { _ in
            .ok(.init(headers: .init(ETag: "\"v1\""), body: .json([
                .init(collector_number: "123", french_name: nil, name: "Elesh Norn", rarity: "R", set_code: "MOM"),
                .init(collector_number: "45", french_name: nil, name: "Reprieve", rarity: "C", set_code: "MOM"),
            ])))
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let model = ScanIndexModel(api: api, store: ScanIndexStore(file: directory.appendingPathComponent("i.json")))
        await model.refresh()
        return model
    }

    private func scanner(api: FakeAPI) async -> ScannerViewModel {
        ScannerViewModel(index: await index(), storage: MemoryStorage(), prices: ScanBatchPrices(api: api))
    }

    private func scan(_ lines: [String], into scanner: ScannerViewModel, from start: TimeInterval) {
        for step in 0..<3 { scanner.frame(lines: lines, at: start + Double(step) * 0.2) }
        // The card leaves the camera, so the same card can be read again.
        scanner.frame(lines: [], at: start + 1)
        scanner.frame(lines: [], at: start + 3)
    }

    @Test func pricesALineInItsFinishAndSumsTheBatch() async {
        let scanner = await scanner(api: FakeAPI.trendPrices(["123": (1000, 5000), "45": (20, nil)]))
        scan(elesh, into: scanner, from: 0)
        scanner.foil = true
        scan(reprieve, into: scanner, from: 5)
        scanner.foil = false
        // Another card in between: the same card read twice in a row counts once.
        scan(elesh, into: scanner, from: 10)
        scan(reprieve, into: scanner, from: 15)
        await scanner.refreshPrices()

        let eleshLine = scanner.batch.lines.first { $0.set == "MOM" && $0.number == "123" }!
        let foilReprieve = scanner.batch.lines.first { $0.number == "45" && $0.foil }!
        let reprieveLine = scanner.batch.lines.first { $0.number == "45" && !$0.foil }!
        #expect(scanner.lineCents(of: eleshLine) == 2000)
        #expect(scanner.lineCents(of: reprieveLine) == 20)
        #expect(scanner.lineCents(of: foilReprieve) == nil)
        #expect(scanner.batchValue == BatchValue(cents: 2020, unpricedLines: 1))
    }

    @Test func aLinePriceFollowsItsQuantityAndFinish() async {
        let scanner = await scanner(api: FakeAPI.trendPrices(["123": (1000, 5000)]))
        scan(elesh, into: scanner, from: 0)
        await scanner.refreshPrices()
        let line = scanner.batch.lines[0]

        scanner.setQuantity(3, ofLine: line.id)

        #expect(scanner.lineCents(of: scanner.batch.lines[0]) == 3000)
        #expect(scanner.batchValue?.cents == 3000)
    }

    @Test func asksForEachNewCardAndHidesPricesWhenTheNetworkFails() async {
        let api = FakeAPI.trendPrices(["123": (1000, nil)])
        let scanner = await scanner(api: api)
        scan(elesh, into: scanner, from: 0)
        await scanner.refreshPrices()
        #expect(scanner.batchValue?.cents == 1000)

        api.onGetTrendPrices = { _ in throw URLError(.notConnectedToInternet) }
        scan(reprieve, into: scanner, from: 5)
        await scanner.refreshPrices()

        #expect(scanner.batchValue == nil)
        #expect(scanner.lineCents(of: scanner.batch.lines[0]) == nil)
        #expect(scanner.batch.cardCount == 2)
    }

    @Test func asksNothingForAnEmptyBatch() async {
        let api = FakeAPI.trendPrices([:])
        let scanner = await scanner(api: api)

        await scanner.refreshPrices()

        #expect(api.calls.isEmpty)
    }

    private let line123 = ScanBatchLine(set: "MOM", number: "123", language: .french, foil: false, quantity: 1,
                                        rarity: "R", name: "Elesh Norn", frenchName: nil)

    @Test func unitCentsIsOneCopyInItsFinish() async {
        let prices = ScanBatchPrices(api: FakeAPI.trendPrices(["123": (1000, 5000)]))
        let key = CardKey(set: "MOM", number: "123")
        await prices.refresh(for: [line123])

        #expect(prices.unitCents(of: key, foil: false) == 1000)
        #expect(prices.unitCents(of: key, foil: true) == 5000)
    }

    @Test func unitCentsIsNilForAnUnknownCardOrFinish() async {
        let prices = ScanBatchPrices(api: FakeAPI.trendPrices(["123": (1000, nil)]))
        await prices.refresh(for: [line123])

        #expect(prices.unitCents(of: CardKey(set: "MOM", number: "123"), foil: true) == nil)
        #expect(prices.unitCents(of: CardKey(set: "MOM", number: "999"), foil: false) == nil)
    }

    @Test func unitCentsIsNilBeforeTheAnswerAndWhenTheNetworkFails() async {
        let api = FakeAPI.trendPrices(["123": (1000, nil)])
        let prices = ScanBatchPrices(api: api)
        let key = CardKey(set: "MOM", number: "123")
        let lines = [line123]
        #expect(prices.unitCents(of: key, foil: false) == nil)

        await prices.refresh(for: lines)
        #expect(prices.unitCents(of: key, foil: false) == 1000)

        api.onGetTrendPrices = { _ in throw URLError(.notConnectedToInternet) }
        await prices.refresh(for: lines)
        #expect(prices.unitCents(of: key, foil: false) == nil)
    }
}
