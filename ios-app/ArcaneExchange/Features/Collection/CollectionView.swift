import SwiftUI

struct CollectionView: View {
    @State private var model = CollectionViewModel()
    @State private var isShowingFilters = false
    @State private var isShowingImport = false
    @State private var path = NavigationPath()
    @Namespace private var cardTransition

    var body: some View {
        NavigationStack(path: $path) {
            content
                .navigationTitle("Ma collection")
                .navigationBarTitleDisplayMode(.inline)
                .accountToolbar()
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button(action: { isShowingImport = true }, label: {
                            Label("Importer", systemImage: "square.and.arrow.down")
                        })
                    }
                }
                .refreshable { await model.reload() }
                .task { await model.loadInitiallyIfNeeded() }
                .task { await model.loadSetsIfNeeded() }
                .onChange(of: model.filters) {
                    Task { await model.reload() }
                }
                .navigationDestination(for: CardDetailRoute.self) { route in
                    CardDetailView(card: route.card)
                        .navigationTransition(.zoom(sourceID: route.card.scryfall_id, in: cardTransition))
                }
                .navigationDestination(for: CardOffersRoute.self) { route in
                    CardOffersView(card: route.card)
                }
                .tradeDestinations()
                .sheet(isPresented: $isShowingFilters) {
                    CollectionFiltersSheet(filters: $model.filters, sets: model.sets)
                }
                .sheet(isPresented: $isShowingImport, onDismiss: handleImportDismiss) {
                    ImportView()
                }
        }
        .tradeNavigation(path: $path)
    }

    @ViewBuilder private var content: some View {
        if model.isLoading, model.cards.isEmpty {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = model.loadError, model.cards.isEmpty {
            CollectionErrorView(error: error) {
                Task { await model.reload() }
            }
        } else if model.cards.isEmpty {
            CollectionEmptyView(hasActiveFilters: model.filters.activeCount > 0) {
                model.filters.clearAll()
            }
        } else {
            CollectionCardGrid(
                cards: model.cards,
                isLoadingMore: model.isLoadingMore,
                summary: summary,
                filters: $model.filters,
                cardTransition: cardTransition,
                onFilterTap: { isShowingFilters = true },
                onCardAppear: { card in await model.loadMoreIfNeeded(displaying: card) }
            )
        }
    }

    private var summary: String {
        "\(CollectionCopy.cardCount(model.total)) · triées par \(model.filters.sortBy.label.lowercased())"
    }

    /// The import may have replaced the whole collection — reload from page 0 once the sheet
    /// closes, whether it completed, failed, or was dismissed mid-import.
    private func handleImportDismiss() {
        Task { await model.reload() }
    }
}
