import APIClient
import Testing

@testable import ArcaneExchangeCore

private func binder(_ id: String, _ name: String, copies: Int64 = 0, isOpen: Bool = false) -> Binder {
    Binder(copies: copies, id: id, is_open: isOpen, name: name)
}

private func listing(_ binders: [Binder]) -> Operations.list_binders.Output {
    .ok(.init(body: .json(.init(binders: binders))))
}

@MainActor
struct BindersViewModelTests {
    @Test func loadsTheBindersEmptyOnesIncluded() async {
        let api = FakeAPI()
        api.onListBinders = { _ in listing([binder("1", "Bulk"), binder("2", "Trade", copies: 4, isOpen: true)]) }
        let model = BindersViewModel(api: api)

        await model.load()

        #expect(model.binders.map(\.name) == ["Bulk", "Trade"])
        #expect(model.loadError == nil)
        #expect(!model.isLoading)
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onListBinders = { _ in throw APIError(code: "internal", status: 500) }
        let model = BindersViewModel(api: api)

        await model.load()

        #expect(model.loadError == "Erreur technique, réessaie plus tard.")
        #expect(model.binders.isEmpty)
    }

    @Test func createsABinderAndKeepsTheListSorted() async {
        let api = FakeAPI()
        api.onListBinders = { _ in listing([binder("1", "Bulk"), binder("2", "Trade")]) }
        nonisolated(unsafe) var sent: String?
        api.onCreateBinder = { input in
            if case let .json(body) = input.body { sent = body.name }
            return .created(.init(body: .json(binder("3", "Foils"))))
        }
        let model = BindersViewModel(api: api)
        await model.load()

        let created = await model.create(name: "Foils")

        #expect(created)
        #expect(sent == "Foils")
        #expect(model.binders.map(\.name) == ["Bulk", "Foils", "Trade"])
        #expect(model.writeError == nil)
    }

    @Test func translatesATakenName() async {
        let api = FakeAPI()
        api.onCreateBinder = { _ in throw APIError(code: "binder_name_taken", status: 409) }
        let model = BindersViewModel(api: api)

        let created = await model.create(name: "Bulk")

        #expect(!created)
        #expect(model.writeError == "Tu as déjà un binder avec ce nom.")
        #expect(model.binders.isEmpty)
    }

    @Test func translatesAnInvalidName() async {
        let api = FakeAPI()
        api.onCreateBinder = { _ in throw APIError(code: "invalid_binder_name", status: 400) }
        let model = BindersViewModel(api: api)

        let created = await model.create(name: "   ")

        #expect(!created)
        #expect(model.writeError == "Le nom d'un binder ne peut pas être vide ni dépasser 255 caractères.")
    }

    @Test func renamesABinderAndKeepsItsTradeOpening() async {
        let api = FakeAPI()
        let trade = binder("2", "Trade", copies: 4, isOpen: true)
        api.onListBinders = { _ in listing([binder("1", "Bulk"), trade]) }
        nonisolated(unsafe) var sentID: String?
        nonisolated(unsafe) var sentName: String?
        api.onRenameBinder = { input in
            sentID = input.path.id
            if case let .json(body) = input.body { sentName = body.name }
            return .ok(.init(body: .json(binder("2", "Aaa", copies: 4, isOpen: true))))
        }
        let model = BindersViewModel(api: api)
        await model.load()

        let renamed = await model.rename(trade, to: "Aaa")

        #expect(renamed)
        #expect(sentID == "2")
        #expect(sentName == "Aaa")
        #expect(model.binders.map(\.name) == ["Aaa", "Bulk"])
        #expect(model.binders.first?.is_open == true)
    }

    @Test func deletesAnEmptyBinder() async {
        let api = FakeAPI()
        let bulk = binder("1", "Bulk")
        api.onListBinders = { _ in listing([bulk, binder("2", "Trade", copies: 4)]) }
        nonisolated(unsafe) var deletedID: String?
        api.onDeleteBinder = { input in
            deletedID = input.path.id
            return .noContent(.init())
        }
        let model = BindersViewModel(api: api)
        await model.load()

        await model.delete(bulk)

        #expect(deletedID == "1")
        #expect(model.binders.map(\.name) == ["Trade"])
    }

    @Test func keepsTheBinderWhenItsDeletionIsRefused() async {
        let api = FakeAPI()
        let full = binder("2", "Trade", copies: 4)
        api.onListBinders = { _ in listing([full]) }
        api.onDeleteBinder = { _ in throw APIError(code: "binder_not_empty", status: 409) }
        let model = BindersViewModel(api: api)
        await model.load()

        await model.delete(full)

        #expect(model.binders.map(\.name) == ["Trade"])
        #expect(model.writeError == "Ce binder contient encore des cartes : déplace-les ou retire-les d'abord.")
    }

    @Test func describesABinderRow() {
        #expect(BinderCopy.detail(copies: 0, isOpen: false) == "Vide")
        #expect(BinderCopy.detail(copies: 1, isOpen: false) == "1 exemplaire")
        #expect(BinderCopy.detail(copies: 1240, isOpen: true) == "1\u{202F}240 exemplaires · ouvert à l'échange")
    }
}
