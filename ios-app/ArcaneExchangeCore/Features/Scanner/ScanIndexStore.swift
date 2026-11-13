import Foundation

/// The scan index on the iPhone's disk, with the ETag it came with. One file, replaced atomically,
/// so the index and its ETag never disagree.
package struct ScanIndexStore: Sendable {
    package struct Snapshot: Codable, Sendable {
        package let etag: String?
        package let entries: [ScanIndexEntry]
    }

    private let file: URL

    package init(file: URL) {
        self.file = file
    }

    /// In Application Support: the system never purges it, and the scanner must work offline.
    package static var standard: ScanIndexStore {
        let directory = URL.applicationSupportDirectory
        return ScanIndexStore(file: directory.appendingPathComponent("scan-index.json"))
    }

    /// Nil when nothing was stored yet, or the file is unreadable (the next download replaces it).
    package func load() -> Snapshot? {
        guard let data = try? Data(contentsOf: file) else { return nil }
        return try? JSONDecoder().decode(Snapshot.self, from: data)
    }

    /// Best effort: a disk that cannot be written leaves the previous index in place.
    package func save(etag: String?, entries: [ScanIndexEntry]) {
        guard let data = try? JSONEncoder().encode(Snapshot(etag: etag, entries: entries)) else { return }
        let directory = file.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try? data.write(to: file, options: .atomic)
    }
}
