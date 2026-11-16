import APIClient
import CoreSpotlight
import Foundation
import Synchronization
import Testing

@testable import ArcaneExchangeCore

/// The index as Spotlight would hold it: entities by key, never read back from the indexer.
private actor FakeIndex: ImpressionIndex {
    private(set) var entities: [ImpressionKey: ImpressionEntity] = [:]
    var keys: Set<ImpressionKey> { Set(entities.keys) }

    private var whileIndexing: (@Sendable () async -> Void)?

    /// Runs `action` while the next `index` is under way, before its write lands.
    func duringNextIndexing(_ action: @escaping @Sendable () async -> Void) {
        whileIndexing = action
    }

    func index(_ impressions: [ImpressionEntity]) async {
        if let action = whileIndexing {
            whileIndexing = nil
            await action()
        }
        // The write lands after the purge, as it may in Core Spotlight.
        for entity in impressions { entities[entity.key] = entity }
    }

    func remove(_ keys: [ImpressionKey]) {
        for key in keys { entities[key] = nil }
    }

    func removeAll() {
        entities = [:]
    }
}

/// The thumbnails as files that exist or not, without any disk or network.
private final class FakeThumbnails: ThumbnailStore {
    private struct State {
        var files: Set<ImpressionKey> = []
        var unavailable: Set<ImpressionKey> = []
    }

    private let state = Mutex(State())

    var files: Set<ImpressionKey> { state.withLock { $0.files } }

    /// The download of this impression's image fails.
    func failDownload(of key: ImpressionKey) {
        state.withLock { _ = $0.unavailable.insert(key) }
    }

    func file(for key: ImpressionKey) -> URL {
        URL(filePath: "/thumbnails/\(key.set)-\(key.number).jpg")
    }

    func thumbnail(for key: ImpressionKey, from _: URL) async -> URL? {
        let isAvailable = state.withLock { state in
            if state.unavailable.contains(key) { return false }
            state.files.insert(key)
            return true
        }
        return isAvailable ? file(for: key) : nil
    }

    func remove(_ keys: [ImpressionKey]) {
        state.withLock { $0.files.subtract(keys) }
    }

    func removeAll() {
        state.withLock { $0.files = [] }
    }
}

private func key(_ number: String) -> ImpressionKey {
    ImpressionKey(set: "FDN", number: number)
}

private func impression(
    _ number: String,
    name: String = "Sol Ring",
    frenchName: String? = nil,
    set: String = "FDN"
) -> SearchCard {
    var card = searchCard("id-\(set)-\(number)")
    card.name = name
    card.french_name = frenchName
    card.set_code = set
    card.collector_number = number
    return card
}

private func page(_ items: [SearchCard], total: Int, number: Int = 0) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: Int32(number), page_size: 100, total: Int64(total)))))
}

/// A Collection served page by page, 100 at a time, as the real search does.
private func serve(_ cards: [SearchCard], on api: FakeAPI, queries: Recorder? = nil) {
    api.onSearchCards = { input in
        queries?.add(input.query)
        let number = Int(input.query.page ?? 0)
        let size = Int(input.query.page_size ?? 20)
        let slice = cards.dropFirst(number * size).prefix(size)
        return page(Array(slice), total: cards.count, number: number)
    }
}

private final class Recorder: Sendable {
    private let recorded = Mutex<[Operations.search_cards.Input.Query]>([])

    var queries: [Operations.search_cards.Input.Query] { recorded.withLock { $0 } }

    func add(_ query: Operations.search_cards.Input.Query) {
        recorded.withLock { $0.append(query) }
    }
}

private final class Clock: Sendable {
    private let current: Mutex<Date>

    init(_ now: Date) {
        current = Mutex(now)
    }

    var now: Date { current.withLock { $0 } }

    func advance(_ seconds: TimeInterval) {
        current.withLock { $0.addTimeInterval(seconds) }
    }
}

struct SpotlightIndexerTests {
    private func makeIndexer(
        _ api: FakeAPI,
        _ index: FakeIndex,
        thumbnails: FakeThumbnails = FakeThumbnails(),
        defaults: UserDefaults = isolatedDefaults(),
        now: (@Sendable () -> Date)? = nil
    ) -> SpotlightIndexer {
        // By default every call happens an hour after the previous one: the wait never gets in the way.
        let hourly = Clock(Date(timeIntervalSince1970: 1_000_000))
        return SpotlightIndexer(api: api, index: index, thumbnails: thumbnails, defaults: defaults, now: now ?? {
            hourly.advance(3600)
            return hourly.now
        })
    }

    @Test func walksEveryPageOfTheOwnedImpressions() async {
        let api = FakeAPI()
        let recorder = Recorder()
        let cards = (1 ... 250).map { impression(String($0)) }
        serve(cards, on: api, queries: recorder)
        let index = FakeIndex()

        await makeIndexer(api, index).reconcileCollection()

        #expect(await index.keys == Set(cards.map(ImpressionKey.init)))
        #expect(recorder.queries.map(\.page) == [0, 1, 2])
        #expect(recorder.queries.allSatisfy { $0.owned == true && $0.page_size == 100 })
    }

    @Test func stopsAtTheSearchDepthLimit() async {
        let api = FakeAPI()
        let recorder = Recorder()
        api.onSearchCards = { input in
            recorder.add(input.query)
            let number = Int(input.query.page ?? 0)
            return page([impression("\(number)")], total: 20000, number: number)
        }

        await makeIndexer(api, FakeIndex()).reconcileCollection()

        #expect(recorder.queries.count == 100)
    }

    @Test func removesTheImpressionsThatLeftTheCollection() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let indexer = makeIndexer(api, index)
        serve([impression("1"), impression("2")], on: api)
        await indexer.reconcileCollection()

        serve([impression("1"), impression("3")], on: api)
        await indexer.reconcileCollection()

        #expect(await index.keys == [key("1"), key("3")])
    }

    @Test func remembersWhatItIndexedAcrossLaunches() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let defaults = isolatedDefaults()
        serve([impression("1"), impression("2")], on: api)
        await makeIndexer(api, index, defaults: defaults).reconcileCollection()

        serve([impression("1")], on: api)
        await makeIndexer(api, index, defaults: defaults).reconcileCollection()

        #expect(await index.keys == [key("1")])
    }

    @Test func leavesTheIndexAsItIsWhenTheAPIFails() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let indexer = makeIndexer(api, index)
        serve([impression("1"), impression("2")], on: api)
        await indexer.reconcileCollection()

        // The second page fails: nothing indexed so far may drop the first run's impressions.
        api.onSearchCards = { input in
            if input.query.page == 0 { return page([impression("9")], total: 150) }
            throw URLError(.notConnectedToInternet)
        }
        await indexer.reconcileCollection()

        #expect(await index.keys == [key("1"), key("2")])
    }

    @Test func leavesTheIndexAsItIsWhenTheSessionHasExpired() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let indexer = makeIndexer(api, index)
        serve([impression("1"), impression("2")], on: api)
        await indexer.reconcileCollection()

        api.onSearchCards = { _ in throw APIError(code: "unauthorized", status: 401) }
        await indexer.reconcileCollection()

        #expect(await index.keys == [key("1"), key("2")])
    }

    @Test func reconcilesAtMostOnceEveryFifteenMinutes() async {
        let api = FakeAPI()
        let recorder = Recorder()
        serve([impression("1")], on: api, queries: recorder)
        let clock = Clock(Date(timeIntervalSince1970: 1_000_000))
        let indexer = makeIndexer(api, FakeIndex(), now: { clock.now })

        await indexer.reconcileCollection()
        clock.advance(14 * 60)
        await indexer.reconcileCollection()
        #expect(recorder.queries.count == 1)

        clock.advance(61)
        await indexer.reconcileCollection()
        #expect(recorder.queries.count == 2)
    }

    @Test func callsAtTheSameTimeWalkTheCollectionOnce() async {
        let api = FakeAPI()
        let recorder = Recorder()
        serve([impression("1")], on: api, queries: recorder)
        let indexer = makeIndexer(api, FakeIndex(), now: { Date(timeIntervalSince1970: 1_000_000) })

        async let launch: Void = indexer.reconcileCollection()
        async let activation: Void = indexer.reconcileCollection()
        _ = await (launch, activation)

        #expect(recorder.queries.count == 1)
    }

    @Test func aFailedReconciliationDoesNotStartTheWait() async {
        let api = FakeAPI()
        let recorder = Recorder()
        api.onSearchCards = { _ in throw URLError(.timedOut) }
        let indexer = makeIndexer(api, FakeIndex(), now: { Date(timeIntervalSince1970: 1_000_000) })
        await indexer.reconcileCollection()

        serve([impression("1")], on: api, queries: recorder)
        await indexer.reconcileCollection()

        #expect(recorder.queries.count == 1)
    }

    @Test func purgeEmptiesTheIndexAndForgetsWhatItIndexed() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let thumbnails = FakeThumbnails()
        let indexer = makeIndexer(api, index, thumbnails: thumbnails)
        serve([impression("1")], on: api)
        await indexer.reconcileCollection()

        await indexer.purge()

        #expect(await index.keys.isEmpty)
        #expect(thumbnails.files.isEmpty)
        // Right after a purge, the next reconciliation is not held back by the wait.
        serve([impression("2")], on: api)
        await indexer.reconcileCollection()
        #expect(await index.keys == [key("2")])
    }

    @Test func aPurgeDuringAReconciliationIsNotUndoneByIt() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let thumbnails = FakeThumbnails()
        let indexer = makeIndexer(api, index, thumbnails: thumbnails)
        api.onSearchCards = { _ in
            // The player signs out while the Collection is being read.
            await indexer.purge()
            return page([impression("1")], total: 1)
        }

        await indexer.reconcileCollection()

        #expect(await index.keys.isEmpty)
        // Nor did it start the wait: the next player's reconciliation walks at once.
        serve([impression("2")], on: api)
        await indexer.reconcileCollection()
        #expect(await index.keys == [key("2")])
    }

    @Test func aPurgeDuringTheIndexingLeavesNothingOfWhatWasWritten() async {
        let api = FakeAPI()
        let index = FakeIndex()
        let thumbnails = FakeThumbnails()
        let indexer = makeIndexer(api, index, thumbnails: thumbnails)
        var card = impression("1")
        card.image_url = "https://img.example/1.jpg"
        serve([card], on: api)
        await index.duringNextIndexing { await indexer.purge() }

        await indexer.reconcileCollection()

        #expect(await index.keys.isEmpty)
        #expect(thumbnails.files.isEmpty)
    }

    // MARK: Thumbnails

    @Test func indexesTheNamesTheSetTheNumberAndTheThumbnailOnly() async throws {
        let api = FakeAPI()
        let index = FakeIndex()
        let thumbnails = FakeThumbnails()
        var card = impression("263", name: "Sol Ring", frenchName: "Anneau solaire", set: "C21")
        card.set_name = "Commander 2021"
        card.image_url = "https://img.example/sol-ring.jpg"
        card.owned_quantity = 4
        card.price_guide = .init(trend: 123)
        serve([card], on: api)

        await makeIndexer(api, index, thumbnails: thumbnails).reconcileCollection()

        let attributes = try #require(await index.entities.values.first?.attributeSet)
        #expect(attributes.displayName == "Anneau solaire")
        #expect(attributes.keywords == ["Sol Ring"])
        #expect(attributes.contentDescription == "Commander 2021 · C21 · 263")
        #expect(attributes.thumbnailURL == thumbnails.file(for: ImpressionKey(set: "C21", number: "263")))
        #expect(attributes.thumbnailURL?.isFileURL == true)
        let everything = [attributes.displayName, attributes.contentDescription, attributes.keywords?.joined()]
            .compactMap(\.self).joined()
        #expect(!everything.contains("123"))
        #expect(!everything.contains("4"))
    }

    @Test func indexesWithoutThumbnailWhenTheDownloadFails() async throws {
        let api = FakeAPI()
        let index = FakeIndex()
        let thumbnails = FakeThumbnails()
        thumbnails.failDownload(of: key("1"))
        var card = impression("1")
        card.image_url = "https://img.example/sol-ring.jpg"
        serve([card], on: api)

        await makeIndexer(api, index, thumbnails: thumbnails).reconcileCollection()

        let attributes = try #require(await index.entities.values.first?.attributeSet)
        #expect(attributes.thumbnailURL == nil)
    }

    @Test func removesTheThumbnailsOfTheImpressionsThatLeftTheCollection() async {
        let api = FakeAPI()
        let thumbnails = FakeThumbnails()
        let indexer = makeIndexer(api, FakeIndex(), thumbnails: thumbnails)
        var first = impression("1")
        var second = impression("2")
        first.image_url = "https://img.example/1.jpg"
        second.image_url = "https://img.example/2.jpg"
        serve([first, second], on: api)
        await indexer.reconcileCollection()

        serve([first], on: api)
        await indexer.reconcileCollection()

        #expect(thumbnails.files == [key("1")])
    }

    @Test func hasNoKeywordWhenTheEnglishNameIsTheDisplayedOne() async throws {
        let api = FakeAPI()
        let index = FakeIndex()
        serve([impression("1", name: "Sol Ring")], on: api)

        await makeIndexer(api, index).reconcileCollection()

        let attributes = try #require(await index.entities.values.first?.attributeSet)
        #expect(attributes.displayName == "Sol Ring")
        #expect(attributes.keywords == nil)
    }
}

struct FileThumbnailStoreTests {
    private func makeStore() -> (FileThumbnailStore, URL) {
        let directory = URL.temporaryDirectory.appending(path: "thumbnails-\(UUID().uuidString)", directoryHint: .isDirectory)
        return (FileThumbnailStore(directory: directory), directory)
    }

    private func write(_ key: ImpressionKey, in directory: URL) throws -> URL {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let file = directory.appending(path: key.id.replacingOccurrences(of: "/", with: "_"))
        try Data("image".utf8).write(to: file)
        return file
    }

    @Test func doesNotDownloadAnImageItAlreadyHas() async throws {
        let (store, directory) = makeStore()
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = try write(key("1"), in: directory)

        // The remote cannot be reached: the file there is the answer.
        let thumbnail = await store.thumbnail(for: key("1"), from: URL(filePath: "/nowhere.jpg"))

        #expect(thumbnail == file)
    }

    @Test func hasNoThumbnailWhenTheDownloadFails() async {
        let (store, directory) = makeStore()
        defer { try? FileManager.default.removeItem(at: directory) }

        let thumbnail = await store.thumbnail(for: key("1"), from: URL(filePath: "/nowhere.jpg"))

        #expect(thumbnail == nil)
    }

    @Test func removesTheFilesOfTheKeysAndOfEverything() throws {
        let (store, directory) = makeStore()
        defer { try? FileManager.default.removeItem(at: directory) }
        let first = try write(key("1"), in: directory)
        let second = try write(key("2"), in: directory)

        store.remove([key("1")])
        #expect(!FileManager.default.fileExists(atPath: first.path))
        #expect(FileManager.default.fileExists(atPath: second.path))

        store.removeAll()
        #expect(!FileManager.default.fileExists(atPath: second.path))
    }
}
