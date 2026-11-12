import APIClient
import Foundation
import Testing

@testable import ArcaneCore

private func importStatus(
    _ id: String = "imp-1",
    status: String,
    errorCode: String? = nil,
    total: Int32 = 10,
    processed: Int32 = 10
) -> Components.Schemas.CardImportResponse {
    .init(
        created_at: "2026-10-01T10:00:00Z",
        error_code: errorCode,
        error_count: 0,
        errors: [],
        id: id,
        processed_lines: processed,
        source_lines: total,
        status: status,
        total_lines: total
    )
}

@MainActor
struct ImportViewModelTests {
    @Test func startsAtThePickStep() {
        let model = ImportViewModel(api: FakeAPI())
        #expect(model.step == .pick)
        #expect(model.loadError == nil)
        #expect(model.wasAlreadyRunning == false)
    }

    @Test func progressFractionIsZeroWithNoStatusYet() {
        #expect(ImportViewModel(api: FakeAPI()).progressFraction == 0)
    }

    @Test func resetReturnsToThePickStepAndClearsState() {
        let model = ImportViewModel(api: FakeAPI())
        model.reset()

        #expect(model.step == .pick)
        #expect(model.status == nil)
        #expect(model.loadError == nil)
        #expect(model.wasAlreadyRunning == false)
    }

    @Test func hasNoFailureMessageBeforeAnyImport() {
        #expect(ImportViewModel(api: FakeAPI()).failureMessage == nil)
    }

    @Test func aRejectedFileShowsTheMessageOfItsCode() async {
        let api = FakeAPI()
        api.onImportCards = { _ in throw APIError(code: "binder_export", status: 400) }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")

        #expect(
            model.loadError
                == "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox."
        )
        #expect(model.step == .pick)
    }

    @Test func aRejectedFileWithAnUnknownCodeShowsTheGenericMessage() async {
        let api = FakeAPI()
        api.onImportCards = { _ in throw APIError(code: "wrong_format", status: 400) }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")

        #expect(model.loadError == APIErrorMessages.generic)
    }

    @Test func anUnreachableServerShowsTheNetworkMessage() async {
        let api = FakeAPI()
        api.onImportCards = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")

        #expect(model.loadError == APIErrorMessages.network)
    }

    @Test func followsTheImportItStarted() async {
        let api = FakeAPI()
        api.onImportCards = { _ in .accepted(.init(body: .json(.init(id: "imp-1")))) }
        api.onGetCardImport = { _ in .ok(.init(body: .json(importStatus(status: "completed")))) }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")
        for _ in 0 ..< 50 where model.step != .done {
            try? await Task.sleep(for: .milliseconds(20))
        }

        #expect(model.step == .done)
        #expect(model.status?.id == "imp-1")
        #expect(model.progressFraction == 1)
        #expect(model.failureMessage == nil)
        #expect(api.calls == ["import_cards", "get_card_import"])
    }

    @Test func aFailedImportShowsTheMessageOfItsErrorCode() async {
        let api = FakeAPI()
        api.onImportCards = { _ in .accepted(.init(body: .json(.init(id: "imp-1")))) }
        api.onGetCardImport = { _ in
            .ok(.init(body: .json(importStatus(status: "failed", errorCode: "no_valid_line"))))
        }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")
        for _ in 0 ..< 50 where model.step != .done {
            try? await Task.sleep(for: .milliseconds(20))
        }

        #expect(model.failureMessage == "Aucune ligne du fichier n'a pu être lue.")
    }

    @Test(arguments: [nil, "some_future_code"] as [String?])
    func aFailedImportWithAnUnknownCodeShowsTheImportGenericMessage(errorCode: String?) async {
        let api = FakeAPI()
        api.onImportCards = { _ in .accepted(.init(body: .json(.init(id: "imp-1")))) }
        api.onGetCardImport = { _ in
            .ok(.init(body: .json(importStatus(status: "failed", errorCode: errorCode))))
        }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")
        for _ in 0 ..< 50 where model.step != .done {
            try? await Task.sleep(for: .milliseconds(20))
        }

        #expect(model.failureMessage == "L'import a échoué.")
    }

    @Test func aConflictFollowsTheImportAlreadyRunning() async {
        let api = FakeAPI()
        api.onImportCards = { _ in throw APIError(code: "import_already_running", status: 409) }
        api.onListCardImports = { _ in
            .ok(.init(body: .json([importStatus("old", status: "completed"), importStatus("run", status: "running")])))
        }
        api.onGetCardImport = { input in
            .ok(.init(body: .json(importStatus(input.path.id, status: "completed"))))
        }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")
        for _ in 0 ..< 50 where model.step != .done {
            try? await Task.sleep(for: .milliseconds(20))
        }

        #expect(model.wasAlreadyRunning)
        #expect(model.status?.id == "run")
        #expect(model.loadError == nil)
    }

    private func apiWithCollection(totalCards: Int64) -> FakeAPI {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in
            .ok(.init(body: .json(.init(binders: [], sets: [], total_cards: totalCards, unique_cards: totalCards))))
        }
        return api
    }

    @Test func anEmptyCollectionOpensThePickerWithoutConfirmation() async {
        let model = ImportViewModel(api: apiWithCollection(totalCards: 0))

        await model.requestImport()

        #expect(model.isPickingFile)
        #expect(!model.isConfirmingReplace)
    }

    @Test func aNonEmptyCollectionAsksForConfirmationBeforeThePicker() async {
        let model = ImportViewModel(api: apiWithCollection(totalCards: 3))

        await model.requestImport()

        #expect(model.isConfirmingReplace)
        #expect(!model.isPickingFile)
    }

    @Test func confirmingTheReplacementOpensThePicker() async {
        let model = ImportViewModel(api: apiWithCollection(totalCards: 3))

        await model.requestImport()
        model.confirmReplace()

        #expect(!model.isConfirmingReplace)
        #expect(model.isPickingFile)
    }

    @Test func cancellingTheConfirmationNeverStartsTheImport() async {
        let api = apiWithCollection(totalCards: 3)
        let model = ImportViewModel(api: api)

        await model.requestImport()
        model.cancelReplace()

        #expect(!model.isConfirmingReplace)
        #expect(!model.isPickingFile)
        #expect(model.step == .pick)
        #expect(!api.calls.contains("import_cards"))
    }

    @Test func anUnreadableCollectionOpensNothing() async {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = ImportViewModel(api: api)

        await model.requestImport()

        #expect(model.loadError == APIErrorMessages.network)
        #expect(!model.isConfirmingReplace)
        #expect(!model.isPickingFile)
    }

    @Test func aConflictWithNoActiveImportShowsTheAlreadyRunningMessage() async {
        let api = FakeAPI()
        api.onImportCards = { _ in throw APIError(code: "import_already_running", status: 409) }
        api.onListCardImports = { _ in .ok(.init(body: .json([importStatus(status: "completed")]))) }
        let model = ImportViewModel(api: api)

        await model.start(csv: "x")

        #expect(model.loadError == APIErrorMessages.message(for: "import_already_running"))
    }
}
