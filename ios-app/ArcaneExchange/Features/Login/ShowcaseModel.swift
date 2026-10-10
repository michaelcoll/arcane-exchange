import Foundation
import Observation
import UIKit

/// The showcase (« Vitrine ») behind the login screen: the images of the most expensive cards
/// held on the platform, from the public `GET /showcase`.
///
/// It never reports an error: an unavailable showcase (offline, failure, empty platform) just
/// leaves `images` empty and the login screen keeps its plain background.
@MainActor
@Observable
final class ShowcaseModel {
    /// The cards of the wall, most expensive first. Empty until a showcase can be shown.
    private(set) var images: [UIImage] = []

    /// Whether the wall fades in: one whose images came over the network does, one restored
    /// from the disk cache is there at once.
    private(set) var fadesIn = false

    private let defaults: UserDefaults
    private let source: ShowcaseSource

    /// The image paths of the last showcase fetched. Public data, kept across sign-outs.
    private static let pathsKey = "showcase_image_paths"

    init(defaults: UserDefaults = .standard, source: ShowcaseSource = .live) {
        self.defaults = defaults
        self.source = source
    }

    /// Shows the showcase of the previous launch, as far as its images are in the artwork
    /// caches: the wall is there at once, and nothing is downloaded — this runs while the
    /// session is still loading, for a player who may turn out to be signed in.
    func restore() async {
        guard images.isEmpty else { return }
        let urls = Self.urls(defaults.stringArray(forKey: Self.pathsKey) ?? [])
        await show(source.cachedImages(urls), fadesIn: false)
    }

    /// Fetches the current showcase and keeps it for the next launch. It goes on screen only
    /// when no wall is shown yet: a wall already drifting is not swapped under the player's
    /// eyes, and the new images are only downloaded to disk, for the next launch to restore.
    func refresh() async {
        guard let paths = await source.paths() else { return }
        defaults.set(paths, forKey: Self.pathsKey)
        let urls = Self.urls(paths)
        guard images.isEmpty else {
            source.warm(urls)
            return
        }
        await show(source.images(urls), fadesIn: true)
    }

    private func show(_ loaded: [UIImage], fadesIn: Bool) {
        // A cancelled load is a partial one.
        guard images.isEmpty, !loaded.isEmpty, !Task.isCancelled else { return }
        self.fadesIn = fadesIn
        images = loaded
    }

    private static func urls(_ paths: [String]) -> [URL] {
        paths.compactMap { CardArtwork.url(imagePath: $0) }
    }
}

/// Where the showcase comes from: the public API for its list, the artwork pipeline for its
/// images.
struct ShowcaseSource {
    /// The image paths of the current showcase, `nil` when it cannot be read.
    var paths: @MainActor () async -> [String]?
    /// The images the artwork caches already hold, without any network.
    var cachedImages: @MainActor ([URL]) async -> [UIImage]
    /// The images, downloaded when the caches do not hold them.
    var images: @MainActor ([URL]) async -> [UIImage]
    /// Downloads to the disk cache, in the background, the images a later launch will show.
    var warm: @MainActor ([URL]) -> Void

    static let live = ShowcaseSource(
        paths: {
            guard case let .ok(response) = try? await APIClientProvider.anonymous.get_showcase() else {
                return nil
            }
            return try? response.body.json
        },
        cachedImages: { await ArtworkPipeline.cachedImages(for: $0) },
        images: { await ArtworkPipeline.images(for: $0) },
        warm: { ArtworkPipeline.warmDiskCache($0) }
    )
}
