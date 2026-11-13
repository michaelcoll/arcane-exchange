import APIClient
import Testing

@testable import ArcaneExchangeCore

private let card = CatalogCard(
    collector_number: "H13",
    french_name: nil,
    image_url: nil,
    name: "Lightning Bolt",
    rarity_code: "R",
    set: .init(code: "MKM", name: "Murders at Karlov Manor")
)

private let added = Operations.add_collection_entry.Output.created(.init(body: .json(.init(
    entries: [],
    reserved_quantity: .init(foil: 0, normal: 0)
))))

@MainActor
struct AddCollectionEntryViewModelTests {
    @Test func theLanguageIsFrenchUntilThePlayerChoosesAnother() {
        let model = AddCollectionEntryViewModel(card: card, api: FakeAPI(), defaults: isolatedDefaults())

        #expect(model.language == .french)
        #expect(model.quantity == 1)
        #expect(!model.foil)
        #expect(model.binderID == nil)
    }

    @Test func sendsTheChoicesOfTheForm() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.AddCollectionEntryRequest?
        api.onAddCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return added
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())
        model.language = .japanese
        model.foil = true
        model.quantity = 4
        model.binderID = "b-1"

        let accepted = await model.add()

        #expect(accepted)
        #expect(sent == .init(
            binder_id: "b-1",
            collector_number: "H13",
            foil: true,
            language_code: "JA",
            quantity: 4,
            set_code: "MKM"
        ))
    }

    @Test func aCopyOutOfAnyBinderSendsNoBinder() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.AddCollectionEntryRequest?
        api.onAddCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return added
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())

        _ = await model.add()

        #expect(sent?.binder_id == nil)
    }

    @Test func theLastLanguageChosenIsTheNextDefault() async {
        let defaults = isolatedDefaults()
        let api = FakeAPI()
        api.onAddCollectionEntry = { _ in added }
        let first = AddCollectionEntryViewModel(card: card, api: api, defaults: defaults)
        first.language = .german

        _ = await first.add()
        let second = AddCollectionEntryViewModel(card: card, api: api, defaults: defaults)

        #expect(second.language == .german)
    }

    @Test func aRefusedAdditionKeepsTheLanguageAndShowsTheTranslatedError() async {
        let defaults = isolatedDefaults()
        let api = FakeAPI()
        api.onAddCollectionEntry = { _ in throw APIError(code: "card_not_in_catalog", status: 404) }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: defaults)
        model.language = .italian

        let accepted = await model.add()

        #expect(!accepted)
        #expect(model.writeError == "Cette carte n'existe pas dans le catalogue.")
        #expect(AddCollectionEntryViewModel(card: card, api: api, defaults: defaults).language == .french)
    }

    @Test func offersTheBindersOfThePlayer() async {
        let api = FakeAPI()
        api.onListBinders = { _ in
            .ok(.init(body: .json(.init(binders: [.init(copies: 3, id: "b-1", is_open: false, name: "Trade")]))))
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())

        await model.loadBinders()

        #expect(model.binders.map(\.name) == ["Trade"])
    }

    @Test func offersTheSealedProductsOfThePlayerMostRecentFirst() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Int32?
        api.onListPurchases = { input in
            asked = input.query.page_size
            return .ok(.init(body: .json(.init(
                items: [purchase("2", on: "2026-11-13"), purchase("1", on: "2026-11-01")],
                page: 0,
                page_size: 100,
                total: 2
            ))))
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())

        await model.loadSealedProducts()

        #expect(model.sealedProducts.map(\.id) == ["2", "1"])
        #expect(asked == 100)
        #expect(model.sealedProductID == nil)
    }

    @Test func aFailedReadLeavesNoSealedProductToChoose() async {
        let api = FakeAPI()
        api.onListPurchases = { _ in throw APIError(code: "internal", status: 500) }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())

        await model.loadSealedProducts()

        #expect(model.sealedProducts.isEmpty)
    }

    @Test func attachesTheCopiesToTheChosenSealedProduct() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.AddCollectionEntryRequest?
        api.onAddCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return added
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())
        model.sealedProductID = "p-1"

        let accepted = await model.add()

        #expect(accepted)
        #expect(sent?.sealed_product_id == "p-1")
    }

    @Test func withoutASealedProductNoneIsSent() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.AddCollectionEntryRequest?
        api.onAddCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return added
        }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())

        _ = await model.add()

        #expect(sent?.sealed_product_id == nil)
    }

    @Test func aSealedProductThatIsGoneIsRefusedWithATranslatedMessage() async {
        let api = FakeAPI()
        api.onAddCollectionEntry = { _ in throw APIError(code: "purchase_not_found", status: 404) }
        let model = AddCollectionEntryViewModel(card: card, api: api, defaults: isolatedDefaults())
        model.sealedProductID = "p-1"

        let accepted = await model.add()

        #expect(!accepted)
        #expect(model.writeError == "Cet achat n'existe plus.")
    }
}
