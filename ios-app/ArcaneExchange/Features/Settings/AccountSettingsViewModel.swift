import APIClient
import Foundation
import OpenAPIRuntime

/// Backs the Réglages sheet: the collection's visibility, the ManaBox binders opened to trade,
/// and the per-rarity rules — the same three endpoints the web client's `pages/profile` and
/// `Profile/TradeRules.vue` drive.
@MainActor
@Observable
final class AccountSettingsViewModel {
    /// Everything a failed request can say, in the user's terms — same shape as
    /// `CollectionViewModel.LoadError`.
    enum RequestError: Equatable {
        case unauthorized
        case network
        case http(Int)
        case unexpected

        var message: String {
            switch self {
            case .unauthorized:
                "Session expirée. Reconnecte-toi pour modifier tes réglages."
            case .network:
                "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."
            case let .http(status):
                "Le serveur a répondu \(status)."
            case .unexpected:
                "Réponse inattendue du serveur."
            }
        }
    }

    private(set) var visibility: CollectionVisibility = ._private
    private(set) var binders: [BinderInfo] = []
    private(set) var selectedBinders: Set<String> = []
    private(set) var rarities: [RarityFilter] = []

    private(set) var isLoading = false
    /// Set when the initial load failed: the screen has nothing to show, so it replaces it.
    private(set) var loadError: RequestError?
    /// Set when a *write* failed: the screen is intact, only the change was rejected.
    var writeError: RequestError?

    private(set) var isSavingVisibility = false
    /// Name of the binder / code of the rarity whose row is waiting on the server, or `nil`.
    private(set) var busyBinder: String?
    private(set) var busyRarity: String?

    var ratio: TradeRatio {
        TradeRatio(rarities: rarities)
    }

    /// Only the rules screen's own data is worth blocking on; the four requests are
    /// independent, so they go out together.
    func load() async {
        isLoading = true
        loadError = nil
        do {
            async let visibility = fetchVisibility()
            async let binders = fetchBinders()
            async let selected = fetchSelectedBinders()
            async let rarities = fetchRarityFilters()
            let loaded = try await (visibility, binders, selected, rarities)
            self.visibility = loaded.0
            self.binders = loaded.1
            selectedBinders = loaded.2
            self.rarities = loaded.3
        } catch {
            loadError = Self.requestError(from: error)
        }
        isLoading = false
    }

    /// Applied optimistically then rolled back on failure, like the web client's `watch(vis)`:
    /// the segmented control must not lag a tap behind the server.
    func setVisibility(_ newValue: CollectionVisibility) async {
        let previous = visibility
        guard newValue != previous else { return }
        visibility = newValue
        isSavingVisibility = true
        defer { isSavingVisibility = false }
        do {
            switch try await APIClientProvider.shared.set_visibility(body: .json(.init(visibility: newValue))) {
            case .noContent:
                break
            case .unauthorized:
                throw APIClientError.unauthorized
            case .badRequest:
                throw APIClientError.undocumented(statusCode: 400)
            case .notFound:
                throw APIClientError.undocumented(statusCode: 404)
            case let .undocumented(statusCode, _):
                throw APIClientError.undocumented(statusCode: statusCode)
            }
        } catch {
            visibility = previous
            writeError = Self.requestError(from: error)
        }
    }

    /// Selecting a binder changes which copies exist per rarity, so the rules are refetched —
    /// the counts on every rarity row belong to the new perimeter.
    func toggleBinder(_ name: String) async {
        let previous = selectedBinders
        let wasSelected = selectedBinders.contains(name)
        if wasSelected {
            selectedBinders.remove(name)
        } else {
            selectedBinders.insert(name)
        }
        busyBinder = name
        defer { busyBinder = nil }
        do {
            if wasSelected {
                try await removeBinder(name)
            } else {
                try await addBinder(name)
            }
        } catch {
            // Only the write itself rolls the toggle back. The refetch below must not: the
            // binder *is* selected server-side, and flipping the row back would have the user
            // undo a change that actually went through.
            selectedBinders = previous
            writeError = Self.requestError(from: error)
            return
        }
        await refreshRarities()
    }

    /// Writes one rarity rule, then refetches: `proposed` is computed server-side from the
    /// copies owned minus the kept ones, and the row shows it.
    func setRarity(_ rarity: String, isOpen: Bool, keptCopies: Int32) async {
        let kept = min(TradeRules.maxKeptCopies, max(0, keptCopies))
        busyRarity = rarity
        defer { busyRarity = nil }
        do {
            let body = Components.Schemas.SetRarityFilterRequest(
                is_open: isOpen,
                kept_copies: kept,
                rarity: rarity
            )
            switch try await APIClientProvider.shared.set_rarity_filter(body: .json(body)) {
            case .noContent:
                break
            case .unauthorized:
                throw APIClientError.unauthorized
            case .badRequest:
                throw APIClientError.undocumented(statusCode: 400)
            case let .undocumented(statusCode, _):
                throw APIClientError.undocumented(statusCode: statusCode)
            }
        } catch {
            writeError = Self.requestError(from: error)
        }
        // Refetched whatever happened, like the web client's `finally`: on success `proposed`
        // is recomputed server-side, and on a rejected write the row must stop showing a value
        // the server never accepted.
        await refreshRarities()
    }

    /// Silent on purpose: this only ever follows a write the user already got feedback on, and
    /// the next `load()` resyncs anyway — a second alert here would just stack on the first.
    private func refreshRarities() async {
        if let refreshed = try? await fetchRarityFilters() {
            rarities = refreshed
        }
    }

    // MARK: Requests

    private func fetchVisibility() async throws -> CollectionVisibility {
        switch try await APIClientProvider.shared.get_visibility() {
        case let .ok(response):
            return try response.body.json.visibility
        case .unauthorized:
            throw APIClientError.unauthorized
        case .notFound:
            throw APIClientError.undocumented(statusCode: 404)
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func fetchBinders() async throws -> [BinderInfo] {
        switch try await APIClientProvider.shared.get_collection_stats() {
        case let .ok(response):
            return try response.body.json.binders
                .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func fetchSelectedBinders() async throws -> Set<String> {
        switch try await APIClientProvider.shared.get_trade_binders() {
        case let .ok(response):
            return try Set(response.body.json.binders)
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func fetchRarityFilters() async throws -> [RarityFilter] {
        switch try await APIClientProvider.shared.get_rarity_filters() {
        case let .ok(response):
            return try response.body.json.rarities
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func addBinder(_ name: String) async throws {
        switch try await APIClientProvider.shared.add_trade_binder(body: .json(.init(binder_name: name))) {
        case .noContent:
            return
        case .unauthorized:
            throw APIClientError.unauthorized
        case .badRequest:
            throw APIClientError.undocumented(statusCode: 400)
        case .notFound:
            throw APIClientError.undocumented(statusCode: 404)
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func removeBinder(_ name: String) async throws {
        switch try await APIClientProvider.shared.remove_trade_binder(path: .init(name: name)) {
        case .noContent:
            return
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private static func requestError(from error: Error) -> RequestError {
        if let apiError = error as? APIClientError {
            switch apiError {
            case .unauthorized: return .unauthorized
            case let .undocumented(statusCode): return .http(statusCode)
            }
        }
        // The transport wraps URLSession failures — a wrong base URL or no network lands here.
        let underlying = (error as? ClientError)?.underlyingError ?? error
        return underlying is URLError ? .network : .unexpected
    }
}
