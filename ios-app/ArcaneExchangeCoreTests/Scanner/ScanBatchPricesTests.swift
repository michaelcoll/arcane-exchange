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

    /// The API answers with the given trend and foil trend for each card, nil meaning unknown.
    private func api(_ prices: [String: (Int32?, Int32?)]) -> FakeAPI {
        let api = FakeAPI()
        api.onGetTrendPrices = { input in
            guard case let .json(request) = input.body else { throw URLError(.badServerResponse) }
            return .ok(.init(body: .json(.init(prices: request.cards.map {
                .init(collector_number: $0.collector_number, foil_trend_cents: prices[$0.collector_number]?.1,
                      set_code: $0.set_code, trend_cents: prices[$0.collector_number]?.0)
            }))))
        }
        return api
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
        let scanner = await scanner(api: api(["123": (1000, 5000), "45": (20, nil)]))
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
        let scanner = await scanner(api: api(["123": (1000, 5000)]))
        scan(elesh, into: scanner, from: 0)
        await scanner.refreshPrices()
        let line = scanner.batch.lines[0]

        scanner.setQuantity(3, ofLine: line.id)

        #expect(scanner.lineCents(of: scanner.batch.lines[0]) == 3000)
        #expect(scanner.batchValue?.cents == 3000)
    }

    @Test func asksForEachNewCardAndHidesPricesWhenTheNetworkFails() async {
        let api = api(["123": (1000, nil)])
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
        let api = api([:])
        let scanner = await scanner(api: api)

        await scanner.refreshPrices()

        #expect(api.calls.isEmpty)
    }
}
