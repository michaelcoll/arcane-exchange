import APIClient
import ArcaneExchangeCore
import SwiftUI

/// A player's tradable collection (`ScrResults` in the mockup), pushed from the player search:
/// a paginated card grid with its filter rail and axes.
struct SearchResultsView: View {
    let route: SearchResultsRoute

    @State private var model: SearchResultsViewModel

    init(route: SearchResultsRoute, api: any APIProtocol) {
        self.route = route
        _model = State(initialValue: SearchResultsViewModel(target: route.target, api: api))
    }

    var body: some View {
        SearchResultsGrid(model: model, showsInlineControls: true)
            .task { await model.loadInitiallyIfNeeded() }
            .navigationTitle(route.title)
            .navigationBarTitleDisplayMode(.inline)
    }
}

/// The card grid of a search — same layout as the Collection tab. With `showsInlineControls`,
/// the filter rail and the axes scroll above the cards; without, the screen offers them
/// elsewhere (the "…" menu of the Rechercher tab).
struct SearchResultsGrid: View {
    @Bindable var model: SearchResultsViewModel
    let showsInlineControls: Bool

    @State private var isShowingFilters = false

    private let columns = [GridItem(.adaptive(minimum: 140), spacing: 14)]

    private var target: SearchResultsRoute.Target {
        model.target
    }

    var body: some View {
        content
            .task(id: model.axes) { await model.loadSetsIfNeeded() }
            .onChange(of: model.filters) {
                Task { await model.paginator.reload() }
            }
            .onChange(of: model.axes) {
                Task { await model.paginator.reload() }
            }
            .sheet(isPresented: $isShowingFilters) {
                CollectionFiltersSheet(filters: $model.filters, sets: model.sets)
            }
    }

    @ViewBuilder private var content: some View {
        if model.isAwaitingQuery, model.paginator.items.isEmpty {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            phaseContent
        }
    }

    @ViewBuilder private var phaseContent: some View {
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

    /// With filters or axes on, the rail and the axes stay above the message: the way out may be
    /// another rarity or another axis, not only clearing the filters.
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
        if showsInlineControls, state.offersClearFilters || model.axes != SearchAxes() {
            VStack(alignment: .leading, spacing: 14) {
                if case let .player(username) = target {
                    playerHeader(username)
                }
                rail
                axesBar
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

    /// The Possession axis, and the Offre axis of a search by name: two independent pickers.
    private var axesBar: some View {
        VStack(spacing: 8) {
            Picker("Possession", selection: $model.axes.possession) {
                ForEach(PossessionAxis.allCases, id: \.self) { Text($0.label) }
            }
            if target.offersOfferAxis {
                Picker("Offre", selection: $model.axes.offer) {
                    ForEach(OfferAxis.allCases, id: \.self) { Text($0.label) }
                }
            }
        }
        .pickerStyle(.segmented)
    }

    private var grid: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 14) {
                if case let .player(username) = target {
                    playerHeader(username)
                }

                if showsInlineControls {
                    rail
                    axesBar
                }

                Text(CollectionCopy.sortedSummary(total: model.paginator.total, sortBy: model.filters.sortBy))
                    .font(.caption)
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)

                LazyVGrid(columns: columns, spacing: 18) {
                    ForEach(model.paginator.items, id: \.self) { card in
                        NavigationLink(value: CardDetailRoute(card: card.asCollectionCard)) {
                            SearchCardCell(card: card)
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
            route: SearchResultsRoute(target: .player(username: "jace")),
            api: APIClientFactory.preview
        )
    }
}
