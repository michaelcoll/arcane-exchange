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
            .unexpected,
        ]

        for error in errors {
            #expect(!error.message.isEmpty)
        }
    }

    @Test func conflictMessageMentionsAnAlreadyRunningImport() {
        #expect(ImportViewModel.LoadError.conflict.message.contains("déjà en cours"))
    }
}
