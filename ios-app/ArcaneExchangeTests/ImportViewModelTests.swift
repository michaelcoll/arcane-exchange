import Testing

@testable import ArcaneExchange

@MainActor
struct ImportViewModelTests {
    @Test func startsAtThePickStep() {
        let model = ImportViewModel()
        #expect(model.step == .pick)
        #expect(model.loadError == nil)
        #expect(model.wasAlreadyRunning == false)
    }

    @Test func progressFractionIsZeroWithNoStatusYet() {
        let model = ImportViewModel()
        #expect(model.progressFraction == 0)
    }

    @Test func resetReturnsToThePickStepAndClearsState() {
        let model = ImportViewModel()
        model.reset()

        #expect(model.step == .pick)
        #expect(model.status == nil)
        #expect(model.loadError == nil)
        #expect(model.wasAlreadyRunning == false)
    }

    @Test func everyLoadErrorHasANonEmptyMessage() {
        let errors: [ImportViewModel.LoadError] = [
            .unauthorized,
            .conflict,
            .network,
            .http(500),
            .rejected(code: "binder_export"),
            .rejected(code: nil),
            .unexpected,
        ]

        for error in errors {
            #expect(!error.message.isEmpty)
        }
    }

    @Test func aRejectedFileShowsTheMessageOfItsCode() {
        #expect(
            ImportViewModel.LoadError.rejected(code: "binder_export").message
                == "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox."
        )
    }

    @Test func aRejectedFileWithAnUnknownCodeShowsAGenericMessage() {
        #expect(ImportViewModel.LoadError.rejected(code: "wrong_format").message == "L'import a échoué.")
    }

    @Test func aRejectedFileNeverShowsTheBareStatus() {
        #expect(!ImportViewModel.LoadError.rejected(code: nil).message.contains("400"))
    }

    @Test func hasNoFailureMessageBeforeAnyImport() {
        #expect(ImportViewModel().failureMessage == nil)
    }

    @Test func conflictMessageMentionsAnAlreadyRunningImport() {
        #expect(ImportViewModel.LoadError.conflict.message.contains("déjà en cours"))
    }
}
