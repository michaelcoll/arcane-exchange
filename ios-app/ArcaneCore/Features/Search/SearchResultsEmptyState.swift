import ArcaneCore

/// What a search results screen says when nothing comes back.
///
/// Kept off the views on purpose: a `View` is `@MainActor`, and pure logic hung on one crashes
/// the non-isolated tests calling it.
package enum SearchResultsEmptyState: Equatable {
    /// Nobody offers a matching card at all.
    case noMatch
    /// The filters emptied the list: they can be cleared, so the screen offers to.
    case noMatchWithFilters

    /// Only the filters count, not the sort: a sort reorders the list, it never empties it.
    package init(filters: CollectionFilters) {
        self = filters.activeCount > 0 ? .noMatchWithFilters : .noMatch
    }

    package var title: String {
        switch self {
        case .noMatch: "Aucun résultat"
        case .noMatchWithFilters: "Aucun résultat avec ces filtres"
        }
    }

    package var message: String? {
        switch self {
        case .noMatch: "Personne ne propose de carte correspondant à cette recherche."
        case .noMatchWithFilters: nil
        }
    }

    package var systemImage: String {
        switch self {
        case .noMatch: "magnifyingglass"
        case .noMatchWithFilters: "line.3.horizontal.decrease"
        }
    }

    package var offersClearFilters: Bool {
        self == .noMatchWithFilters
    }
}
