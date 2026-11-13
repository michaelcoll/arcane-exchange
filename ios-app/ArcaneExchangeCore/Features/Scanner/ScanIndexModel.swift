import APIClient
import Foundation

/// Keeps the scan index on the iPhone up to date and in memory (ADR-0024): downloaded at launch and
/// whenever the Scanner tab opens, with `If-None-Match`. Whatever goes wrong with the download (a
/// `304`, a `503`, no network) leaves the local index in place, with no visible error.
@MainActor
@Observable
package final class ScanIndexModel {
    package enum State: Equatable {
        /// Nothing known yet: the local index is not read and the download has not answered.
        case loading
        case ready
        /// No local index and no download: the player needs the network a first time.
        case needsNetwork
    }

    package private(set) var state: State = .loading
    /// The index the scanner recognises cards against; nil until `state` is `.ready`.
    package private(set) var index: ScanIndex?

    private let api: any APIProtocol
    private let store: ScanIndexStore
    private var etag: String?

    /// `nonisolated` so an environment default can build one.
    package nonisolated init(api: any APIProtocol, store: ScanIndexStore = .standard) {
        self.api = api
        self.store = store
    }

    package func refresh() async {
        if index == nil {
            await loadLocalIndex()
        }
        await download()
        if index == nil {
            state = .needsNetwork
        }
    }

    private func loadLocalIndex() async {
        let store = store
        guard let snapshot = await Task.detached(priority: .userInitiated, operation: { store.load() }).value
        else { return }
        etag = snapshot.etag
        index = await Task.detached { ScanIndex(entries: snapshot.entries) }.value
        state = .ready
    }

    private func download() async {
        let input = Operations.get_scan_index.Input(headers: .init(If_hyphen_None_hyphen_Match: etag))
        // A `304` is not an `.ok`: reading it throws, like a failure, and keeps the local index.
        guard let served = try? await api.get_scan_index(input).ok,
              let entries = try? served.body.json.map(ScanIndexEntry.init)
        else { return }
        let store = store
        let etag = served.headers.ETag
        index = await Task.detached(priority: .userInitiated) {
            store.save(etag: etag, entries: entries)
            return ScanIndex(entries: entries)
        }.value
        self.etag = etag
        state = .ready
    }
}
