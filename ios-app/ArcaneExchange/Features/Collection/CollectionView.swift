import SwiftUI

struct CollectionView: View {
    @State private var model = CollectionViewModel()
    @State private var isShowingFilters = false
    @State private var path = NavigationPath()
    @Namespace private var cardTransition

    private let columns = [GridItem(.adaptive(minimum: 140), spacing: 14)]

    var body: some View {
        NavigationStack(path: $path) {
            content
                .navigationTitle("Ma collection")
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
                .tradeDestinations(path: $path)
                .sheet(isPresented: $isShowingFilters) {
                    CollectionFiltersSheet(
                        filters: $model.filters,
                        sets: model.sets,
                        resultCount: model.total
                    )
                }
        }
    }

    @ViewBuilder private var content: some View {
        if model.isLoading, model.cards.isEmpty {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = model.loadError, model.cards.isEmpty {
            errorView(error)
        } else if model.cards.isEmpty {
            emptyView
        } else {
            grid
        }
    }

    private var grid: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 14) {
                filterRail

                Text(summary)
                    .font(.caption)
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)

                LazyVGrid(columns: columns, spacing: 18) {
                    ForEach(model.cards, id: \.self) { card in
                        NavigationLink(value: CardDetailRoute(card: card)) {
                            CollectionCardCell(card: card)
                        }
                        .buttonStyle(.plain)
                        .matchedTransitionSource(id: card.scryfall_id, in: cardTransition)
                        .task { await model.loadMoreIfNeeded(displaying: card) }
                    }
                }

                if model.isLoadingMore {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 24)
        }
    }

    /// Scrolls with the grid, like the mockup's chip rail — the filters are a refinement of the
    /// list, not app chrome.
    private var filterRail: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 8) {
                sortMenu
                chipButton(
                    title: setsChipTitle,
                    systemImage: "square.stack.3d.up",
                    isActive: !model.filters.sets.isEmpty
                )
                chipButton(
                    title: raritiesChipTitle,
                    systemImage: "line.3.horizontal.decrease",
                    isActive: !model.filters.rarities.isEmpty
                )
            }
            .padding(.vertical, 2)
        }
        .scrollIndicators(.hidden)
        .scrollClipDisabled()
    }

    private var sortMenu: some View {
        Menu(content: {
            Picker("Trier par", selection: $model.filters.sortBy) {
                ForEach(SortField.collectionOptions, id: \.self) { field in
                    Text(field.label).tag(field)
                }
            }
            Picker("Ordre", selection: $model.filters.sortDir) {
                Text("Décroissant").tag(SortDirection.desc)
                Text("Croissant").tag(SortDirection.asc)
            }
        }, label: {
            chipLabel(model.filters.sortBy.label, systemImage: "arrow.up.arrow.down")
        })
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .tint(.accentColor)
    }

    private func chipButton(title: String, systemImage: String, isActive: Bool) -> some View {
        Button(action: { isShowingFilters = true }, label: {
            chipLabel(title, systemImage: systemImage)
        })
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .tint(isActive ? Color.accentColor : Color.secondary)
    }

    private func chipLabel(_ title: String, systemImage: String) -> some View {
        Label(title, systemImage: systemImage)
            .font(.subheadline)
            .fontWeight(.medium)
    }

    private func errorView(_ error: CollectionViewModel.LoadError) -> some View {
        ContentUnavailableView(
            label: { Label("Collection indisponible", systemImage: "exclamationmark.triangle") },
            description: { Text(error.message) },
            actions: {
                Button("Réessayer") {
                    Task { await model.reload() }
                }
            }
        )
    }

    @ViewBuilder private var emptyView: some View {
        if model.filters.activeCount > 0 {
            ContentUnavailableView(
                label: { Label("Aucune carte", systemImage: "line.3.horizontal.decrease") },
                description: { Text("Aucune carte de ta collection ne correspond à ces filtres.") },
                actions: {
                    Button("Réinitialiser les filtres") { model.filters.clearAll() }
                }
            )
        } else {
            ContentUnavailableView(
                "Collection vide",
                systemImage: "rectangle.stack",
                description: Text("Importe ton fichier ManaBox pour voir tes cartes ici.")
            )
        }
    }

    private var summary: String {
        "\(CollectionCopy.cardCount(model.total)) · triées par \(model.filters.sortBy.label.lowercased())"
    }

    private var setsChipTitle: String {
        let count = model.filters.sets.count
        return count == 0 ? "Sets" : "Sets · \(count)"
    }

    private var raritiesChipTitle: String {
        let count = model.filters.rarities.count
        return count == 0 ? "Raretés" : "Raretés · \(count)"
    }
}

#Preview {
    CollectionView()
}
