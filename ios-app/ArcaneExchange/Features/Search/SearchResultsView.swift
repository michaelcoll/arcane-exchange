import APIClient
import SwiftUI

/// Where a submitted search lands (`ScrResults` in the mockup): a paginated card grid for a
/// name or player search, or a placeholder for the not-yet-wired decklist mode.
struct SearchResultsView: View {
    let route: SearchResultsRoute
    let api: any APIProtocol

    var body: some View {
        Group {
            switch route.target {
            case .decklist:
                decklistPlaceholder
            case .card, .player:
                SearchResultsGrid(target: route.target, api: api)
            }
        }
        .navigationTitle(route.title)
        .navigationBarTitleDisplayMode(.inline)
    }

    private var decklistPlaceholder: some View {
        ContentUnavailableView {
            Label("Recherche par decklist à venir", systemImage: "list.bullet.rectangle")
        } description: {
            Text("""
            L'API cherche carte par carte : il faut un endroit côté backend pour grouper la \
            liste avant de brancher ce mode.
            """)
        }
    }
}

/// The card grid for a name or player-scoped search — same layout as the Collection tab.
private struct SearchResultsGrid: View {
    let target: SearchResultsRoute.Target

    @State private var model: SearchResultsViewModel
    @State private var isShowingFilters = false

    private let columns = [GridItem(.adaptive(minimum: 140), spacing: 14)]

    init(target: SearchResultsRoute.Target, api: any APIProtocol) {
        self.target = target
        _model = State(initialValue: SearchResultsViewModel(target: target, api: api))
    }

    var body: some View {
        content
            .task { await model.loadInitiallyIfNeeded() }
            .task { await model.loadSetsIfNeeded() }
            .onChange(of: model.filters) {
                Task { await model.paginator.reload() }
            }
            .sheet(isPresented: $isShowingFilters) {
                CollectionFiltersSheet(filters: $model.filters, sets: model.sets)
            }
    }

    @ViewBuilder private var content: some View {
        switch model.paginator.phase {
        case .firstLoad:
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(error):
            ContentUnavailableView(
                label: { Label("Recherche indisponible", systemImage: "exclamationmark.triangle") },
                description: { Text(error) },
                actions: { Button("Réessayer") { Task { await model.paginator.reload() } } }
            )
        case .empty:
            emptyState(SearchResultsEmptyState(filters: model.filters))
        case .content:
            grid
        }
    }

    /// With filters on, the rail stays above the message: the way out may be another rarity,
    /// not only clearing them all.
    @ViewBuilder private func emptyState(_ state: SearchResultsEmptyState) -> some View {
        let unavailable = ContentUnavailableView(
            label: { Label(state.title, systemImage: state.systemImage) },
            description: {
                if let message = state.message {
                    Text(message)
                }
            },
            actions: {
                if state.offersClearFilters {
                    // Clears the filters only: the sort is kept.
                    Button("Effacer les filtres") { model.filters.clearAll() }
                }
            }
        )
        if state.offersClearFilters {
            VStack(alignment: .leading, spacing: 14) {
                if case let .player(username) = target {
                    playerHeader(username)
                }
                rail
                unavailable
            }
            .padding(.horizontal, 16)
        } else {
            unavailable
        }
    }

    private var rail: some View {
        CollectionFilterRail(
            filters: $model.filters,
            sortOptions: target.sortOptions,
            onFilterTap: { isShowingFilters = true }
        )
    }

    private var grid: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 14) {
                if case let .player(username) = target {
                    playerHeader(username)
                }

                rail

                Text(CollectionCopy.sortedSummary(total: model.paginator.total, sortBy: model.filters.sortBy))
                    .font(.caption)
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)

                LazyVGrid(columns: columns, spacing: 18) {
                    ForEach(model.paginator.items, id: \.self) { card in
                        NavigationLink(value: CardDetailRoute(card: card)) {
                            CollectionCardCell(card: card)
                        }
                        .buttonStyle(.plain)
                        .task { await model.paginator.loadMore(after: card) }
                    }
                }

                if model.paginator.state == .loadingMore {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 24)
        }
    }

    /// No card count here: the line right below already states how many tradable cards came
    /// back.
    private func playerHeader(_ username: String) -> some View {
        HStack(spacing: 12) {
            PlayerAvatar(username: username)
            UsernameLabel(username: username)
                .fontWeight(.semibold)
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
    }
}

#Preview {
    NavigationStack {
        SearchResultsView(
            route: SearchResultsRoute(target: .card(query: "Vampiric Tutor")),
            api: APIClientFactory.preview
        )
    }
}
