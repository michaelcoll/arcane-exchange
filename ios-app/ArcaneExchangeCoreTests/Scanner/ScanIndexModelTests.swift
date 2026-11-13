import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

private func entry(_ set: String, _ number: String) -> Components.Schemas.ScanIndexEntryResponse {
    .init(collector_number: number, name: "Elesh Norn", rarity: "R", set_code: set)
}

private func served(etag: String?, _ entries: [Components.Schemas.ScanIndexEntryResponse])
    -> Operations.get_scan_index.Output {
    .ok(.init(headers: .init(ETag: etag), body: .json(entries)))
}

@MainActor
struct ScanIndexModelTests {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let api = FakeAPI()

    private var store: ScanIndexStore { ScanIndexStore(file: directory.appendingPathComponent("scan-index.json")) }

    private func model() -> ScanIndexModel { ScanIndexModel(api: api, store: store) }

    private func recognises(_ model: ScanIndexModel, set: String = "MOM", number: String = "123") -> Bool {
        guard let index = model.index else { return false }
        return index.entry(set: set, number: number) != nil
    }

    @Test func aFreshIndexIsKeptOnDiskWithItsETag() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        let model = model()

        await model.refresh()

        #expect(model.state == .ready)
        #expect(recognises(model))
        #expect(store.load()?.etag == "\"v1\"")
    }

    @Test func asksForTheIndexWithTheETagItAlreadyHas() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        await model().refresh()
        nonisolated(unsafe) var sent: String?
        api.onGetScanIndex = { input in
            sent = input.headers.If_hyphen_None_hyphen_Match
            return .notModified(.init())
        }

        await model().refresh()

        #expect(sent == "\"v1\"")
    }

    @Test func aNotModifiedAnswerKeepsTheLocalIndex() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        await model().refresh()
        api.onGetScanIndex = { _ in .notModified(.init()) }
        let relaunched = model()

        await relaunched.refresh()

        #expect(relaunched.state == .ready)
        #expect(recognises(relaunched))
        #expect(store.load()?.etag == "\"v1\"")
    }

    @Test func aServiceUnavailableAnswerKeepsTheLocalIndex() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        await model().refresh()
        api.onGetScanIndex = { _ in throw APIError(code: "scan_index_unavailable", status: 503) }
        let relaunched = model()

        await relaunched.refresh()

        #expect(relaunched.state == .ready)
        #expect(recognises(relaunched))
    }

    @Test func aNetworkFailureKeepsTheLocalIndex() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        await model().refresh()
        api.onGetScanIndex = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let relaunched = model()

        await relaunched.refresh()

        #expect(relaunched.state == .ready)
        #expect(recognises(relaunched))
    }

    @Test func aNewerIndexReplacesTheLocalOne() async {
        api.onGetScanIndex = { _ in served(etag: "\"v1\"", [entry("MOM", "123")]) }
        let model = model()
        await model.refresh()
        api.onGetScanIndex = { _ in served(etag: "\"v2\"", [entry("MKM", "12")]) }

        await model.refresh()

        #expect(recognises(model, set: "MKM", number: "12"))
        #expect(!recognises(model))
        #expect(store.load()?.etag == "\"v2\"")
    }

    @Test func withoutALocalIndexANetworkFailureNeedsTheNetwork() async {
        api.onGetScanIndex = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = model()
        #expect(model.state == .loading)

        await model.refresh()

        #expect(model.state == .needsNetwork)
        #expect(model.index == nil)
    }

    @Test func withoutALocalIndexAServiceUnavailableAnswerNeedsTheNetwork() async {
        api.onGetScanIndex = { _ in throw APIError(code: "scan_index_unavailable", status: 503) }
        let model = model()

        await model.refresh()

        #expect(model.state == .needsNetwork)
    }
}
