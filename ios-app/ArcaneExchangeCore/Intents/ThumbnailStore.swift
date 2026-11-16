import Foundation

/// Where the card images Spotlight shows are kept: Core Spotlight reads a thumbnail from a local
/// file, never from a remote URL.
package protocol ThumbnailStore: Sendable {
    /// The local file of the image of an impression, downloaded from `remote` unless it is already
    /// there. `nil` when it cannot be downloaded: the impression is then indexed without it.
    func thumbnail(for key: ImpressionKey, from remote: URL) async -> URL?
    func remove(_ keys: [ImpressionKey])
    func removeAll()
}

/// The thumbnails as files of the app's container, one per impression, named after its key.
package struct FileThumbnailStore: ThumbnailStore {
    private let directory: URL
    private let session: URLSession

    package init(
        directory: URL = URL.applicationSupportDirectory.appending(path: "SpotlightThumbnails"),
        session: URLSession = .shared
    ) {
        self.directory = directory
        self.session = session
    }

    package func thumbnail(for key: ImpressionKey, from remote: URL) async -> URL? {
        let file = file(for: key)
        if FileManager.default.fileExists(atPath: file.path) {
            return file
        }
        do {
            let (data, response) = try await session.data(from: remote)
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { return nil }
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try data.write(to: file, options: .atomic)
            return file
        } catch {
            return nil
        }
    }

    package func remove(_ keys: [ImpressionKey]) {
        for key in keys {
            try? FileManager.default.removeItem(at: file(for: key))
        }
    }

    package func removeAll() {
        try? FileManager.default.removeItem(at: directory)
    }

    /// A set code holds no `/`, a collector number might: the key becomes one file name.
    private func file(for key: ImpressionKey) -> URL {
        directory.appending(path: key.id.replacingOccurrences(of: "/", with: "_"), directoryHint: .notDirectory)
    }
}
