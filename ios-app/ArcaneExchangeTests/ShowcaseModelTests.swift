import Foundation
import Testing
import UIKit

@testable import ArcaneExchange

@MainActor
struct ShowcaseModelTests {
    private let defaults = UserDefaults(suiteName: "ShowcaseModelTests.\(UUID().uuidString)")!
    private let platform = Platform()

    /// What stands in for the API and the artwork caches, and what the model asked of them.
    @MainActor
    private final class Platform {
        /// The image paths `GET /showcase` answers, `nil` when it is unavailable.
        var showcase: [String]?
        /// The images already on the device, by path.
        var cached = [String: UIImage]()
        var downloaded = [String]()
        var warmed = [String]()

        var source: ShowcaseSource {
            ShowcaseSource(
                paths: { self.showcase },
                cachedImages: { urls in urls.compactMap { self.cached[$0.path] } },
                images: { urls in
                    self.downloaded += urls.map(\.path)
                    return urls.map { _ in UIImage() }
                },
                warm: { urls in self.warmed += urls.map(\.path) }
            )
        }
    }

    private func model() -> ShowcaseModel {
        ShowcaseModel(defaults: defaults, source: platform.source)
    }

    /// A launch after `paths` were shown: the list is remembered, the images are on disk.
    private func rememberShowcase(_ paths: [String]) async {
        platform.showcase = paths
        await model().refresh()
        for path in paths {
            platform.cached[path] = UIImage()
        }
        platform.downloaded = []
        platform.warmed = []
    }

    @Test func theWallOfThePreviousLaunchIsThereAtOnceWithoutAnyDownload() async {
        await rememberShowcase(["/card-images/a.webp", "/card-images/b.webp"])
        let model = model()

        await model.restore()

        #expect(model.images.count == 2)
        #expect(!model.fadesIn)
        #expect(platform.downloaded.isEmpty)
    }

    @Test func aFirstWallIsDownloadedAndFadesIn() async {
        platform.showcase = ["/card-images/a.webp"]
        let model = model()

        await model.restore()
        #expect(model.images.isEmpty)
        await model.refresh()

        #expect(model.images.count == 1)
        #expect(model.fadesIn)
        #expect(platform.downloaded == ["/card-images/a.webp"])
    }

    /// The wall on screen is not swapped, and the next launch finds the new one on disk.
    @Test func aNewShowcaseBehindAWallOnScreenIsWarmedForTheNextLaunch() async {
        await rememberShowcase(["/card-images/a.webp"])
        let model = model()
        await model.restore()
        platform.showcase = ["/card-images/b.webp", "/card-images/c.webp"]

        await model.refresh()

        #expect(model.images.count == 1)
        #expect(!model.fadesIn)
        #expect(platform.warmed == ["/card-images/b.webp", "/card-images/c.webp"])
        #expect(platform.downloaded.isEmpty)
    }

    @Test func anUnavailableShowcaseLeavesNoWallAndKeepsTheRememberedOne() async {
        await rememberShowcase(["/card-images/a.webp"])
        platform.showcase = nil
        platform.cached = [:]
        let offline = model()

        await offline.restore()
        await offline.refresh()

        #expect(offline.images.isEmpty)
        platform.cached["/card-images/a.webp"] = UIImage()
        let next = model()
        await next.restore()
        #expect(next.images.count == 1)
    }
}
