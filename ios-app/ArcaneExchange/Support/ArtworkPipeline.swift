import Foundation
import Nuke

/// Nuke's shared pipeline, tuned for card artwork.
enum ArtworkPipeline {
    private static let diskSizeLimit = 512 * 1024 * 1024

    /// Card images never change behind their URL — the platform serves them `immutable` for a
    /// year, under a URL versioned by the origin of the file, which changes whenever the file
    /// is replaced (ADR 0017) — so the aggressive `DataCache` is the right trade over the HTTP
    /// cache: it keys on the URL and keeps the bytes until the size limit evicts them, ignoring
    /// `Cache-Control` entirely.
    ///
    /// What it replaces is `URLCache.shared`, whose defaults (500 KB in memory, 20 MB on
    /// disk) hold about three artworks in RAM — every scroll back up the collection grid
    /// re-downloaded the tiles it had just shown.
    ///
    /// On top of that, `ImageCache.shared` keeps *decoded* images — the thing `AsyncImage`
    /// has no layer for, and why a cached tile still flashed its placeholder on reappearing.
    static func install() {
        ImagePipeline.shared = ImagePipeline(
            configuration: .withDataCache(
                name: "fr.piconsoft.arcane-exchange.artwork",
                sizeLimit: diskSizeLimit
            )
        )
    }

    /// Warms artwork the user has not scrolled to yet.
    ///
    /// The prefetcher runs its requests at `.low` priority, two at a time, so warming never
    /// delays the tile on screen; and the pipeline coalesces a prefetch still in flight with
    /// the `LazyImage` request that catches up with it, rather than fetching twice.
    static func prefetch(_ urls: [URL]) {
        prefetcher.startPrefetching(with: urls)
    }

    /// Drops everything still queued. Worth calling when the pending URLs stop being the ones
    /// the user is about to reach — a filter change replaces the whole result set.
    static func cancelPrefetching() {
        prefetcher.stopPrefetching()
    }

    private static let prefetcher = ImagePrefetcher()
}
