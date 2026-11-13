import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct ScannerViewModelTests {
    let lines = ["Elesh Norn", "R 0123", "MOM • FR", "Jean Dupont"]

    private func index(loaded: Bool) async -> ScanIndexModel {
        let api = FakeAPI()
        api.onGetScanIndex = { _ in
            loaded
                ? .ok(.init(headers: .init(ETag: "\"v1\""), body: .json([
                    .init(collector_number: "123", french_name: "Elesh Norn, Mère des Machines",
                          name: "Elesh Norn", rarity: "R", set_code: "MOM"),
                ])))
                : .notModified(.init())
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let model = ScanIndexModel(api: api, store: ScanIndexStore(file: directory.appendingPathComponent("i.json")))
        await model.refresh()
        return model
    }

    @Test func showsTheCardOnceItIsSeenOnTwoConsecutiveFrames() async {
        let scanner = ScannerViewModel(index: await index(loaded: true))

        scanner.frame(lines: lines, at: 0)
        #expect(scanner.lastRecognised == nil)
        scanner.frame(lines: lines, at: 0.2)

        #expect(scanner.lastRecognised?.displayName == "Elesh Norn, Mère des Machines")
        #expect(scanner.lastRecognised?.set == "MOM")
        #expect(scanner.lastRecognised?.number == "123")
        #expect(scanner.recognitionCount == 1)
    }

    @Test func countsTheCardOnceWhileItStaysInFrontOfTheCamera() async {
        let scanner = ScannerViewModel(index: await index(loaded: true))

        for step in 0..<5 {
            scanner.frame(lines: lines, at: Double(step) * 0.2)
        }

        #expect(scanner.recognitionCount == 1)
    }

    @Test func recognisesNothingWithoutAnIndex() async {
        let scanner = ScannerViewModel(index: await index(loaded: false))

        scanner.frame(lines: lines, at: 0)
        scanner.frame(lines: lines, at: 0.2)

        #expect(scanner.lastRecognised == nil)
        #expect(scanner.recognitionCount == 0)
    }
}
