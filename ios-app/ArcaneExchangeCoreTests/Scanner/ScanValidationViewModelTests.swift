import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct ScanValidationViewModelTests {
    private final class MemoryStorage: ScanBatchStorage {
        var stored: ScanBatch?
        func load() -> ScanBatch? { stored }
        func save(_ batch: ScanBatch) { stored = batch }
    }

    private func card(_ number: String, set: String = "MOM", language: CardLanguage = .french) -> RecognisedCard {
        RecognisedCard(set: set, number: number, language: language, rarity: "C", name: "Card \(number)",
                       frenchName: nil)
    }

    private let foilTwo = ScanBatchLine.Key(set: "MOM", number: "2", language: .french, foil: true)

    /// A scanner holding a batch of `MOM-1` (2 copies, normal) and `MOM-2` (1 foil copy).
    private func scanner() -> ScannerViewModel {
        var batch = ScanBatch()
        batch.addFromCamera(card("1"), foil: false)
        batch.setQuantity(2, ofLine: .init(set: "MOM", number: "1", language: .french, foil: false))
        batch.addFromCamera(card("2"), foil: true)
        let storage = MemoryStorage()
        storage.stored = batch
        let index = ScanIndexModel(api: FakeAPI(), store: ScanIndexStore(file: FileManager.default.temporaryDirectory
                .appendingPathComponent(UUID().uuidString).appendingPathComponent("i.json")))
        return ScannerViewModel(index: index, storage: storage)
    }

    // MARK: Sending

    @Test func sendsTheBatchWithItsIdentifierItsLinesAndTheDestination() async {
        let scanner = scanner()
        let batchID = scanner.batch.id
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.ValidateScanBatchRequest?
        api.onValidateScanBatch = { input in
            if case let .json(body) = input.body { sent = body }
            return .created(.init(body: .json(.init(copies_added: 3))))
        }
        let model = ScanValidationViewModel(scanner: scanner, api: api)
        model.binderID = "b-1"
        model.sealedProductID = "s-1"

        let done = await model.send()

        #expect(done)
        #expect(sent?.batch_id == batchID.uuidString.lowercased())
        #expect(sent?.binder_id == "b-1")
        #expect(sent?.sealed_product_id == "s-1")
        // Most recent line first, as the batch lists them.
        #expect(sent?.lines == [
            .init(collector_number: "2", foil: true, language_code: "FR", quantity: 1, set_code: "MOM"),
            .init(collector_number: "1", foil: false, language_code: "FR", quantity: 2, set_code: "MOM"),
        ])
    }

    @Test func withoutADestinationNoBinderNorSealedProductIsSent() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.ValidateScanBatchRequest?
        api.onValidateScanBatch = { input in
            if case let .json(body) = input.body { sent = body }
            return .created(.init(body: .json(.init(copies_added: 3))))
        }

        _ = await ScanValidationViewModel(scanner: scanner(), api: api).send()

        #expect(sent?.binder_id == nil)
        #expect(sent?.sealed_product_id == nil)
    }

    @Test func aCreatedAnswerEmptiesTheBatchWhichGetsANewIdentifier() async {
        let scanner = scanner()
        let batchID = scanner.batch.id
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in .created(.init(body: .json(.init(copies_added: 3)))) }
        let model = ScanValidationViewModel(scanner: scanner, api: api)

        _ = await model.send()

        #expect(scanner.batch.lines.isEmpty)
        #expect(scanner.batch.id != batchID)
        #expect(model.error == nil)
    }

    @Test func anOkAnswerForABatchAlreadyValidatedIsASuccessToo() async {
        let scanner = scanner()
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in .ok(.init(body: .json(.init(copies_added: 3)))) }

        let done = await ScanValidationViewModel(scanner: scanner, api: api).send()

        #expect(done)
        #expect(scanner.batch.lines.isEmpty)
    }

    // MARK: Failure

    @Test func aFailureKeepsTheBatchAndTellsWhyInFrench() async {
        let scanner = scanner()
        let before = scanner.batch
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in throw APIError(code: "binder_not_found", status: 404) }
        let model = ScanValidationViewModel(scanner: scanner, api: api)

        let done = await model.send()

        #expect(!done)
        #expect(scanner.batch == before)
        #expect(model.error == "Ce binder n'existe plus dans ta collection.")
        #expect(!model.isSending)
    }

    @Test func aNetworkFailureKeepsTheBatchToo() async {
        let scanner = scanner()
        let before = scanner.batch
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in throw URLError(.notConnectedToInternet) }
        let model = ScanValidationViewModel(scanner: scanner, api: api)

        _ = await model.send()

        #expect(scanner.batch == before)
        #expect(model.error == APIErrorMessages.network)
    }

    @Test func aResendCarriesTheSameIdentifier() async {
        let scanner = scanner()
        let api = FakeAPI()
        nonisolated(unsafe) var identifiers: [String] = []
        nonisolated(unsafe) var attempt = 0
        api.onValidateScanBatch = { input in
            if case let .json(body) = input.body { identifiers.append(body.batch_id) }
            attempt += 1
            if attempt == 1 { throw URLError(.timedOut) }
            return .created(.init(body: .json(.init(copies_added: 3))))
        }
        let model = ScanValidationViewModel(scanner: scanner, api: api)

        let first = await model.send()
        let second = await model.send()

        #expect(!first)
        #expect(second)
        #expect(identifiers.count == 2)
        #expect(identifiers[0] == identifiers[1])
        #expect(model.error == nil)
    }

    @Test func theCardsTheCatalogRefusedAreFlaggedInTheList() async {
        let scanner = scanner()
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in
            throw APIError(code: "card_not_in_catalog", status: 404,
                           unknownCards: [.init(collector_number: "2", language_code: "FR", set_code: "MOM")])
        }
        let model = ScanValidationViewModel(scanner: scanner, api: api)

        _ = await model.send()

        #expect(model.rejectedLineIDs == [foilTwo])
        #expect(model.error == "Des cartes du lot n'existent pas dans le catalogue : elles sont signalées dans la liste.")
        #expect(scanner.batch.lines.count == 2)
    }

    @Test func aRefusedLineDisappearsFromTheFlagsOnceRemoved() async {
        let scanner = scanner()
        let api = FakeAPI()
        api.onValidateScanBatch = { _ in
            throw APIError(code: "card_not_in_catalog", status: 404,
                           unknownCards: [.init(collector_number: "2", language_code: "FR", set_code: "MOM")])
        }
        let model = ScanValidationViewModel(scanner: scanner, api: api)
        _ = await model.send()

        scanner.setQuantity(0, ofLine: foilTwo)

        #expect(model.rejectedLineIDs.isEmpty)
    }

    @Test func aSuccessForgetsTheFlagsAndTheDestination() async {
        let scanner = scanner()
        let api = FakeAPI()
        nonisolated(unsafe) var attempt = 0
        api.onValidateScanBatch = { _ in
            attempt += 1
            if attempt == 1 {
                throw APIError(code: "card_not_in_catalog", status: 404,
                               unknownCards: [.init(collector_number: "2", language_code: "FR", set_code: "MOM")])
            }
            return .created(.init(body: .json(.init(copies_added: 1))))
        }
        let model = ScanValidationViewModel(scanner: scanner, api: api)
        model.binderID = "b-1"
        _ = await model.send()
        scanner.setQuantity(0, ofLine: foilTwo)

        _ = await model.send()

        #expect(model.rejectedLineIDs.isEmpty)
        #expect(model.binderID == nil)
        #expect(model.sealedProductID == nil)
    }

    @Test func anEmptyBatchCannotBeSent() {
        let scanner = scanner()
        scanner.clear()

        #expect(!ScanValidationViewModel(scanner: scanner, api: FakeAPI()).canSend)
    }

    // MARK: Destination

    @Test func offersTheBindersAndTheSealedProductsOfThePlayer() async {
        let api = FakeAPI()
        api.onListBinders = { _ in
            .ok(.init(body: .json(.init(binders: [.init(copies: 3, id: "b-1", is_open: false, name: "Trade")]))))
        }
        api.onListPurchases = { _ in
            .ok(.init(body: .json(.init(
                items: [singleCardPurchase("c1"), purchase("s1")], page: 0, page_size: 100, total: 2
            ))))
        }
        let model = ScanValidationViewModel(scanner: scanner(), api: api)

        await model.load()

        #expect(model.binders.map(\.name) == ["Trade"])
        #expect(model.sealedProducts.map(\.id) == ["s1"])
    }

    @Test func choosingNoBinderIsTheDefault() {
        let model = ScanValidationViewModel(scanner: scanner(), api: FakeAPI())

        #expect(model.binderID == nil)
        #expect(model.sealedProductID == nil)
    }
}
