import APIClient
import Foundation

typealias Binder = Components.Schemas.BinderResponse

/// Backs the Binders screen: the player's binders, empty ones included, and their creation,
/// renaming and deletion (`/collection/binders`). The trade opening of a binder is not edited
/// here, it stays in the trade settings.
@MainActor
@Observable
final class BindersViewModel {
    private let api: any APIProtocol

    init(api: any APIProtocol) {
        self.api = api
    }

    private(set) var binders: [Binder] = []
    private(set) var isLoading = false
    /// Set when the load failed and nothing is shown yet. French message, ready to show.
    private(set) var loadError: String?
    /// Set when a write failed: the list is intact, only the change was refused.
    /// French message, ready to show.
    var writeError: String?
    /// Whether a write is waiting on the server: the screen holds still meanwhile.
    private(set) var isWriting = false

    func load() async {
        isLoading = true
        loadError = nil
        do {
            binders = try await api.list_binders().ok.body.json.binders
        } catch {
            loadError = APIError.read(error).message
        }
        isLoading = false
    }

    /// Returns whether the binder was created, so the form that asked can close.
    func create(name: String) async -> Bool {
        await write {
            let created = try await self.api.create_binder(body: .json(.init(name: name))).created.body.json
            self.binders = Self.sorted(self.binders + [created])
        }
    }

    /// Returns whether the binder was renamed, so the form that asked can close.
    func rename(_ binder: Binder, to name: String) async -> Bool {
        await write {
            let renamed = try await self.api.rename_binder(
                path: .init(id: binder.id),
                body: .json(.init(name: name))
            ).ok.body.json
            self.binders = Self.sorted(self.binders.map { $0.id == renamed.id ? renamed : $0 })
        }
    }

    func delete(_ binder: Binder) async {
        _ = await write {
            _ = try await self.api.delete_binder(path: .init(id: binder.id)).noContent
            self.binders.removeAll { $0.id == binder.id }
        }
    }

    /// Runs one write, keeping a refusal in `writeError` for the alert.
    private func write(_ operation: () async throws -> Void) async -> Bool {
        isWriting = true
        defer { isWriting = false }
        do {
            try await operation()
            return true
        } catch {
            writeError = APIError.read(error).message
            return false
        }
    }

    private static func sorted(_ binders: [Binder]) -> [Binder] {
        binders.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }
}

extension APIProtocol {
    /// The player's binders; empty when the read failed, so a caller falls back on "no binder".
    func bindersOrEmpty() async -> [Binder] {
        await (try? list_binders().ok.body.json.binders) ?? []
    }
}
