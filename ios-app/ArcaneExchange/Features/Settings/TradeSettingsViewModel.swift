import APIClient
import Foundation

/// Backs the trade settings of the Réglages sheet: the collection's visibility, the ManaBox
/// binders opened to trade, and the rarity filters — the `/collection/trade-settings`
/// endpoints the web client's `useTradeSettingsService` also drives.
@MainActor
@Observable
final class TradeSettingsViewModel {
    private let api: any APIProtocol

    init(api: any APIProtocol) {
        self.api = api
    }

    private(set) var visibility: CollectionVisibility = ._private
    private(set) var binders: [BinderInfo] = []
    private(set) var selectedBinders: Set<String> = []
    private(set) var rarities: [RarityFilter] = []

    private(set) var isLoading = false
    /// Set when the initial load failed: the screen has nothing to show, so it replaces it.
    /// French message, ready to show.
    private(set) var loadError: String?
    /// Set when a *write* failed: the screen is intact, only the change was rejected.
    /// French message, ready to show.
    var writeError: String?

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
            loadError = APIError.read(error).message
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
            _ = try await api.set_trade_visibility(body: .json(.init(visibility: newValue))).noContent
        } catch {
            visibility = previous
            writeError = APIError.read(error).message
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
            writeError = APIError.read(error).message
            return
        }
        await refreshRarities()
    }

    /// Writes one rarity rule, then refetches: `proposed` is computed server-side from the
    /// copies owned minus the kept ones, and the row shows it.
    func setRarity(_ rarity: String, isOpen: Bool, keptCopies: Int32) async {
        let kept = min(TradeSettings.maxKeptCopies, max(0, keptCopies))
        busyRarity = rarity
        defer { busyRarity = nil }
        do {
            let body = Components.Schemas.SetRarityFilterRequest(
                is_open: isOpen,
                kept_copies: kept,
                rarity: rarity
            )
            _ = try await api.set_trade_rarity_filter(body: .json(body)).noContent
        } catch {
            writeError = APIError.read(error).message
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
        try await api.get_trade_visibility().ok.body.json.visibility
    }

    private func fetchBinders() async throws -> [BinderInfo] {
        try await api.get_collection_stats().ok.body.json.binders
            .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }

    private func fetchSelectedBinders() async throws -> Set<String> {
        try await Set(api.get_trade_binders().ok.body.json.binders)
    }

    private func fetchRarityFilters() async throws -> [RarityFilter] {
        try await api.get_trade_rarity_filters().ok.body.json.rarities
    }

    private func addBinder(_ name: String) async throws {
        _ = try await api.add_trade_binder(body: .json(.init(binder_name: name))).noContent
    }

    private func removeBinder(_ name: String) async throws {
        _ = try await api.remove_trade_binder(path: .init(name: name)).noContent
    }
}
