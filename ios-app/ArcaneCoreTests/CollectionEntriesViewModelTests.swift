import APIClient
import Testing

@testable import ArcaneCore

private let card = CollectionCard(
    collector_number: "1",
    foil: false,
    language_code: "FR",
    name: "Test",
    rarity_code: "C",
    reserved: false,
    scryfall_id: "s-1",
    set_code: "TST"
)

private func entry(_ id: String, foil: Bool = false, quantity: Int32 = 2, binder: String? = nil) -> CollectionEntryDetail {
    CollectionEntryDetail(
        added_at: "2026-01-05T10:00:00Z",
        binder: binder.map { .init(id: "b-\($0)", name: $0) },
        foil: foil,
        id: id,
        purchase_price: nil,
        quantity: quantity
    )
}

private func entries(_ list: [CollectionEntryDetail], reservedNormal: Int32 = 0, reservedFoil: Int32 = 0)
    -> Components.Schemas.CardEntriesResponse
{
    .init(entries: list, reserved_quantity: .init(foil: reservedFoil, normal: reservedNormal))
}

@MainActor
struct CollectionEntriesViewModelTests {
    @Test func loadsTheEntriesOfTheCardWithTheirReservedQuantity() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Operations.get_collection_entries.Input.Path?
        api.onGetCollectionEntries = { input in
            asked = input.path
            return .ok(.init(body: .json(entries([entry("e-1", binder: "A")], reservedNormal: 1))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        await model.load()

        #expect(model.entries.map(\.id) == ["e-1"])
        #expect(model.reserved(foil: false) == 1)
        #expect(model.reserved(foil: true) == 0)
        #expect(asked?.set_code == "TST")
        #expect(asked?.collector_number == "1")
        #expect(asked?.language_code == "FR")
    }

    @Test func knowsOnceTheEntriesWereRead() async {
        let api = FakeAPI()
        api.onGetCollectionEntries = { _ in .ok(.init(body: .json(entries([])))) }
        let model = CollectionEntriesViewModel(card: card, api: api)
        #expect(!model.hasLoaded)

        await model.load()

        // An empty answer is an answer: the card is no longer in the collection.
        #expect(model.hasLoaded)
    }

    @Test func aFailedReadIsNotAnAnswer() async {
        let api = FakeAPI()
        api.onGetCollectionEntries = { _ in throw APIError(code: "internal", status: 500) }
        let model = CollectionEntriesViewModel(card: card, api: api)

        await model.load()

        #expect(!model.hasLoaded)
    }

    @Test func editSendsOnlyWhatChangedAndShowsTheAnswer() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.UpdateCollectionEntryRequest?
        api.onUpdateCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(entries([entry("e-1", quantity: 5)]))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        let accepted = await model.edit(entry("e-1"), quantity: 5, foil: false)

        #expect(accepted)
        #expect(sent?.quantity == 5)
        #expect(sent?.foil == nil)
        #expect(model.entries.map(\.quantity) == [5])
    }

    @Test func editCanChangeTheFinish() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.UpdateCollectionEntryRequest?
        api.onUpdateCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(entries([entry("e-1", foil: true)]))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        _ = await model.edit(entry("e-1"), quantity: 2, foil: true)

        #expect(sent?.foil == true)
        #expect(sent?.quantity == nil)
    }

    @Test func translatesAReservedQuantityRefusalAndKeepsTheList() async {
        let api = FakeAPI()
        api.onGetCollectionEntries = { _ in .ok(.init(body: .json(entries([entry("e-1", quantity: 3)])))) }
        api.onUpdateCollectionEntry = { _ in throw APIError(code: "reserved_quantity_exceeded", status: 409) }
        let model = CollectionEntriesViewModel(card: card, api: api)
        await model.load()

        let accepted = await model.edit(entry("e-1", quantity: 3), quantity: 1, foil: false)

        #expect(!accepted)
        #expect(model.writeError?.contains("réservées") == true)
        #expect(model.entries.map(\.quantity) == [3])
    }

    @Test func removeReloadsTheEntries() async {
        let api = FakeAPI()
        nonisolated(unsafe) var reads = 0
        api.onGetCollectionEntries = { _ in
            reads += 1
            return .ok(.init(body: .json(entries(reads == 1 ? [entry("e-1")] : []))))
        }
        api.onRemoveCollectionEntry = { _ in .noContent(.init()) }
        let model = CollectionEntriesViewModel(card: card, api: api)
        await model.load()

        let removed = await model.remove(entry("e-1"))

        #expect(removed)
        #expect(model.entries.isEmpty)
        #expect(api.calls == ["get_collection_entries", "remove_collection_entry", "get_collection_entries"])
    }

    @Test func loadsTheBindersAsMoveTargets() async {
        let api = FakeAPI()
        api.onListBinders = { _ in
            .ok(.init(body: .json(.init(binders: [Binder(copies: 0, id: "b-1", is_open: false, name: "Bulk")]))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        await model.loadBinders()

        #expect(model.binders.map(\.name) == ["Bulk"])
    }

    @Test func moveSendsTheTargetAndTheQuantityAndShowsTheAnswer() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.MoveCollectionEntryRequest?
        nonisolated(unsafe) var movedID: String?
        api.onMoveCollectionEntry = { input in
            movedID = input.path.id
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(entries([entry("e-1", quantity: 3), entry("e-2", quantity: 1, binder: "B")]))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        let accepted = await model.move(entry("e-1", quantity: 4), to: "b-B", quantity: 1)

        #expect(accepted)
        #expect(movedID == "e-1")
        #expect(sent?.binder_id == "b-B")
        #expect(sent?.quantity == 1)
        #expect(model.entries.map(\.id) == ["e-1", "e-2"])
    }

    @Test func moveOutOfAnyBinderSendsNoBinder() async {
        let api = FakeAPI()
        nonisolated(unsafe) var sent: Components.Schemas.MoveCollectionEntryRequest?
        api.onMoveCollectionEntry = { input in
            if case let .json(body) = input.body { sent = body }
            return .ok(.init(body: .json(entries([entry("e-1")]))))
        }
        let model = CollectionEntriesViewModel(card: card, api: api)

        _ = await model.move(entry("e-1", binder: "A"), to: nil, quantity: 2)

        #expect(sent?.binder_id == nil)
    }

    @Test func translatesAMoveRefusedBecauseTheBinderIsGone() async {
        let api = FakeAPI()
        api.onGetCollectionEntries = { _ in .ok(.init(body: .json(entries([entry("e-1")])))) }
        api.onMoveCollectionEntry = { _ in throw APIError(code: "binder_not_found", status: 404) }
        let model = CollectionEntriesViewModel(card: card, api: api)
        await model.load()

        let accepted = await model.move(entry("e-1"), to: "b-1", quantity: 1)

        #expect(!accepted)
        #expect(model.writeError == "Ce binder n'existe plus dans ta collection.")
        #expect(model.entries.map(\.id) == ["e-1"])
    }

    @Test func translatesAnEntryThatIsGone() async {
        let api = FakeAPI()
        api.onRemoveCollectionEntry = { _ in throw APIError(code: "collection_entry_not_found", status: 404) }
        let model = CollectionEntriesViewModel(card: card, api: api)

        let removed = await model.remove(entry("e-1"))

        #expect(!removed)
        #expect(model.writeError == "Cet exemplaire n'existe plus dans ta collection.")
    }
}
