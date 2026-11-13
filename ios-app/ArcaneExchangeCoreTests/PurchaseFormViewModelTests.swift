import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

private func sets(_ pairs: (String, String)...) -> Operations.list_sets.Output {
    .ok(.init(body: .json(pairs.map { .init(code: $0.0, name: $0.1) })))
}

@MainActor
struct PurchaseFormViewModelTests {
    @Test func loadsTheSetsToPickFrom() async {
        let api = FakeAPI()
        api.onListSets = { _ in sets(("FDN", "Foundations"), ("ECL", "Lorwyn Eclipsed")) }
        let model = PurchaseFormViewModel(api: api)

        await model.loadSets()

        #expect(model.sets.map(\.code) == ["FDN", "ECL"])
        #expect(model.loadError == nil)
    }

    @Test func showsTheTranslatedMessageOfAFailedSetsLoad() async {
        let api = FakeAPI()
        api.onListSets = { _ in throw APIError(code: "internal", status: 500) }
        let model = PurchaseFormViewModel(api: api)

        await model.loadSets()

        #expect(model.loadError == "Erreur technique, réessaie plus tard.")
    }

    @Test func startsOnToday() {
        let model = PurchaseFormViewModel(api: FakeAPI())

        #expect(PurchaseDate.string(from: model.date) == PurchaseDate.string(from: Date()))
    }

    @Test func cannotBeSubmittedWithoutASetOrAPositivePrice() {
        let model = PurchaseFormViewModel(api: FakeAPI())
        #expect(!model.canSubmit)

        model.setCode = "FDN"
        #expect(!model.canSubmit)

        model.priceText = "0"
        #expect(!model.canSubmit)

        model.priceText = "4,99"
        #expect(model.canSubmit)
    }

    @Test func createsASealedProductWithItsAmountInCents() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.CreateSealedProductRequest?
        api.onCreatePurchase = { input in
            if case let .json(body) = input.body { sent = body }
            return .created(.init(body: .json(purchase("1"))))
        }
        let model = PurchaseFormViewModel(api: api)
        model.productType = .PRECONSTRUCTED_DECK
        model.setCode = "FDN"
        model.name = "Deck Bleu"
        model.priceText = "49,99"
        model.date = PurchaseDate.date(from: "2026-11-10")!

        let saved = await model.submit()

        #expect(saved)
        #expect(sent?.product_type == .PRECONSTRUCTED_DECK)
        #expect(sent?.set_code == "FDN")
        #expect(sent?.name == "Deck Bleu")
        #expect(sent?.amount_cents == 4999)
        #expect(sent?.purchased_on == "2026-11-10")
    }

    @Test func leavesTheNameOutWhenItIsEmpty() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.CreateSealedProductRequest?
        api.onCreatePurchase = { input in
            if case let .json(body) = input.body { sent = body }
            return .created(.init(body: .json(purchase("1"))))
        }
        let model = PurchaseFormViewModel(api: api)
        model.setCode = "FDN"
        model.priceText = "5"

        _ = await model.submit()

        #expect(sent?.name == nil)
        #expect(sent?.amount_cents == 500)
    }

    @Test func translatesARefusedCreation() async {
        let api = FakeAPI()
        api.onCreatePurchase = { _ in throw APIError(code: "set_not_found", status: 404) }
        let model = PurchaseFormViewModel(api: api)
        model.setCode = "ZZZ"
        model.priceText = "5"

        let saved = await model.submit()

        #expect(!saved)
        #expect(model.writeError == "Ce set est introuvable.")
        #expect(!model.isSaving)
    }

    @Test func doesNotCallTheAPIWithoutAValidForm() async {
        let api = FakeAPI()
        let model = PurchaseFormViewModel(api: api)

        let saved = await model.submit()

        #expect(!saved)
        #expect(api.calls.isEmpty)
    }

    @Test func editingStartsFromThePurchase() {
        let model = PurchaseFormViewModel(
            api: FakeAPI(),
            editing: purchase("1", type: .DISPLAY, name: "Boîte", set: ("ECL", "Lorwyn Eclipsed"), cents: 12000, on: "2026-10-02")
        )

        #expect(model.isEditing)
        #expect(model.productType == .DISPLAY)
        #expect(model.setCode == "ECL")
        #expect(model.selectedSet?.name == "Lorwyn Eclipsed")
        #expect(model.name == "Boîte")
        #expect(model.priceText == "120")
        #expect(PurchaseDate.string(from: model.date) == "2026-10-02")
    }

    @Test func editingSendsTheFormToThePurchase() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sentID: String?
        nonisolated(unsafe) var sent: Components.Schemas.UpdatePurchaseRequest?
        api.onUpdatePurchase = { input in
            sentID = input.path.id
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(purchase("1"))))
        }
        let model = PurchaseFormViewModel(api: api, editing: purchase("1", name: "Ancien nom"))
        model.name = ""
        model.priceText = "6,50"

        let saved = await model.submit()

        #expect(saved)
        #expect(sentID == "1")
        // An empty name clears the stored one.
        #expect(sent?.name == "")
        #expect(sent?.amount_cents == 650)
        #expect(sent?.set_code == "FDN")
        #expect(api.calls == ["update_purchase"])
    }

    @Test func editingASingleCardPurchaseNeedsOnlyAPriceAndSendsOnlyThePriceAndTheDay() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.UpdatePurchaseRequest?
        api.onUpdatePurchase = { input in
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(singleCardPurchase("1"))))
        }
        let model = PurchaseFormViewModel(api: api, editing: singleCardPurchase("1", cents: 800))

        #expect(model.isSingleCard)
        #expect(model.canSubmit)
        #expect(model.priceText == "8")
        model.priceText = "6,50"
        let saved = await model.submit()

        #expect(saved)
        #expect(sent == .init(amount_cents: 650, purchased_on: "2026-11-13"))
    }

    @Test func aSealedProductIsNotASingleCard() {
        #expect(!PurchaseFormViewModel(api: FakeAPI(), editing: purchase("1")).isSingleCard)
        #expect(!PurchaseFormViewModel(api: FakeAPI()).isSingleCard)
    }

    @Test func translatesARefusedEdit() async {
        let api = FakeAPI()
        api.onUpdatePurchase = { _ in throw APIError(code: "purchase_not_found", status: 404) }
        let model = PurchaseFormViewModel(api: api, editing: purchase("1"))

        let saved = await model.submit()

        #expect(!saved)
        #expect(model.writeError == "Cet achat n'existe plus.")
    }
}
