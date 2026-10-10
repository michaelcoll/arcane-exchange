import Foundation
import Nuke
import Testing
import UIKit

@testable import ArcaneExchange

/// Asserts on `ImagePipeline.shared` rather than on a freshly built configuration: the test
/// host runs `ArcaneExchangeApp.init()` — and so `ArtworkPipeline.install()` — before the
/// suite, so this checks what the app actually loads artwork through.
struct ArtworkPipelineTests {
    private var configuration: ImagePipeline.Configuration {
        ImagePipeline.shared.configuration
    }

    @Test func keepsArtworkInAnAggressiveDiskCache() {
        let cache = configuration.dataCache as? DataCache
        #expect(cache != nil)
        #expect(cache?.sizeLimit == 512 * 1024 * 1024)
    }

    /// The HTTP cache is deliberately out of the loop: `DataCache` alone keeps the artwork.
    @Test func bypassesTheHTTPCache() {
        let loader = configuration.dataLoader as? DataLoader
        #expect(loader != nil)
        #expect(loader?.session.configuration.urlCache == nil)
    }

    /// Decoded images stay in memory, so a tile scrolled back into view does not decode again.
    @Test func keepsDecodedImagesInMemory() {
        #expect(configuration.imageCache != nil)
    }

    /// What the login wall restores at launch, before anybody is known to be signed out.
    @Test func cachedImagesAreReadFromDiskWithoutAnyRequest() async throws {
        let onDisk = try #require(URL(string: "https://example.test/card-images/on-disk.webp"))
        let missing = try #require(URL(string: "https://example.test/card-images/missing.webp"))
        let network = RecordingDataLoader()
        var configuration = ImagePipeline.Configuration(dataLoader: network)
        configuration.dataCache = InMemoryDataCache()
        configuration.imageCache = nil
        let pipeline = ImagePipeline(configuration: configuration)
        let png = UIGraphicsImageRenderer(size: CGSize(width: 2, height: 2)).pngData { _ in }
        pipeline.cache.storeCachedData(png, for: ImageRequest(url: onDisk))

        let images = await ArtworkPipeline.cachedImages(for: [missing, onDisk], in: pipeline)

        #expect(images.count == 1)
        #expect(network.requested.isEmpty)
    }
}

/// A network that answers nothing and remembers what it was asked.
private final class RecordingDataLoader: DataLoading, @unchecked Sendable {
    private let lock = NSLock()
    private var urls = [URL]()

    var requested: [URL] {
        lock.withLock { urls }
    }

    func loadData(
        with request: URLRequest,
        didReceiveData _: @escaping @Sendable (Data, URLResponse) -> Void,
        completion: @escaping @Sendable (Error?) -> Void
    ) -> any Cancellable {
        lock.withLock { urls += [request.url].compactMap(\.self) }
        completion(URLError(.notConnectedToInternet))
        return NoTask()
    }

    private struct NoTask: Cancellable {
        func cancel() {}
    }
}

/// Nuke's `DataCache` writes asynchronously; this one holds what it is given at once.
private final class InMemoryDataCache: DataCaching, @unchecked Sendable {
    private let lock = NSLock()
    private var storage = [String: Data]()

    func cachedData(for key: String) -> Data? {
        lock.withLock { storage[key] }
    }

    func containsData(for key: String) -> Bool {
        lock.withLock { storage[key] != nil }
    }

    func storeData(_ data: Data, for key: String) {
        lock.withLock { storage[key] = data }
    }

    func removeData(for key: String) {
        lock.withLock { storage[key] = nil }
    }

    func removeAll() {
        lock.withLock { storage.removeAll() }
    }
}
