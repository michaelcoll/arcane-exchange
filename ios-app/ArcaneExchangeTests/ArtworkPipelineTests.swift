import Nuke
import Testing

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

    /// The HTTP cache is deliberately out of the loop — `URLCache.shared`'s 20 MB disk
    /// budget held only a fraction of a collection, and its separate entry for Scryfall's
    /// image redirect expired every two days.
    @Test func bypassesTheHTTPCache() {
        let loader = configuration.dataLoader as? DataLoader
        #expect(loader != nil)
        #expect(loader?.session.configuration.urlCache == nil)
    }

    /// The layer `AsyncImage` never had: decoded images, so a tile scrolled back into view
    /// does not decode again.
    @Test func keepsDecodedImagesInMemory() {
        #expect(configuration.imageCache != nil)
    }
}
