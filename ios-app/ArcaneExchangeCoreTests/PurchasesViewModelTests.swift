import APIClient
import Testing

@testable import ArcaneExchangeCore

func purchase(
    _ id: String,
    type: SealedProductType = .BOOSTER,
    name: String? = nil,
    set: (String, String) = ("FDN", "Foundations"),
    cents: Int32 = 499,
    on day: String = "2026-11-13"
) -> Purchase {
    Purchase(
        amount_cents: cents,
        id: id,
        kind: .sealed_product,
        name: name,
        product_type: type,
        purchased_on: day,
        set: .init(code: set.0, name: set.1)
    )
}

private func page(_ items: [Purchase], total: Int64? = nil) -> Operations.list_purchases.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total ?? Int64(items.count)))))
}

@MainActor
struct PurchasesViewModelTests {
    @Test func loadsThePurchasesInTheOrderOfTheAPI() async {
        let api = FakeAPI()
        api.onListPurchases = { _ in page([purchase("2", on: "2026-11-13"), purchase("1", on: "2026-11-01")]) }
        let model = PurchasesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.items.map(\.id) == ["2", "1"])
        #expect(model.paginator.total == 2)
        #expect(model.paginator.phase == .content)
    }

    @Test func asksTheFirstPageWithTheStandardPageSize() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: (Int32?, Int32?)?
        api.onListPurchases = { input in
            asked = (input.query.page, input.query.page_size)
            return page([])
        }
        let model = PurchasesViewModel(api: api)

        await model.reload()

        #expect(asked?.0 == 0)
        #expect(asked?.1 == Paginator<Purchase>.pageSize)
        #expect(model.paginator.phase == .empty)
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onListPurchases = { _ in throw APIError(code: "internal", status: 500) }
        let model = PurchasesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.phase == .failed("Erreur technique, réessaie plus tard."))
    }

    @Test func deletesAPurchaseThenReloadsTheList() async {
        let api = FakeAPI()
        nonisolated(unsafe) var remaining = [purchase("2"), purchase("1")]
        api.onListPurchases = { _ in page(remaining) }
        nonisolated(unsafe) var deletedID: String?
        api.onDeletePurchase = { input in
            deletedID = input.path.id
            remaining.removeAll { $0.id == input.path.id }
            return .noContent(.init())
        }
        let model = PurchasesViewModel(api: api)
        await model.reload()

        await model.delete(purchase("2"))

        #expect(deletedID == "2")
        #expect(model.paginator.items.map(\.id) == ["1"])
        #expect(model.writeError == nil)
    }

    @Test func keepsTheListAndTranslatesARefusedDeletion() async {
        let api = FakeAPI()
        api.onListPurchases = { _ in page([purchase("1")]) }
        api.onDeletePurchase = { _ in throw APIError(code: "purchase_not_found", status: 404) }
        let model = PurchasesViewModel(api: api)
        await model.reload()

        await model.delete(purchase("1"))

        #expect(model.writeError == "Cet achat n'existe plus.")
        #expect(model.paginator.items.map(\.id) == ["1"])
        #expect(!model.isWriting)
    }
}
