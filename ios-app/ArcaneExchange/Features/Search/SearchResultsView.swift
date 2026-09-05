import SwiftUI

/// Where a submitted search lands (`ScrResults` in the mockup): a paginated card grid for a
/// name or player search, or a placeholder for the not-yet-wired decklist mode.
struct SearchResultsView: View {
    let route: SearchResultsRoute

    var body: some View {
        Group {
            switch route.target {
            case .decklist:
                decklistPlaceholder
            case .card, .player:
                SearchResultsGrid(target: route.target)
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

    private let columns = [GridItem(.adaptive(minimum: 140), spacing: 14)]

    init(target: SearchResultsRoute.Target) {
        self.target = target
        _model = State(initialValue: SearchResultsViewModel(target: target))
    }

    var body: some View {
        content
            .task { await model.load() }
    }

    @ViewBuilder private var content: some View {
        if model.isLoading, model.cards.isEmpty {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = model.loadError, model.cards.isEmpty {
            ContentUnavailableView(
                label: { Label("Recherche indisponible", systemImage: "exclamationmark.triangle") },
                description: { Text(error.message) },
                actions: { Button("Réessayer") { Task { await model.load() } } }
            )
        } else if model.cards.isEmpty {
            ContentUnavailableView(
                "Aucun résultat",
                systemImage: "magnifyingglass",
                description: Text("Personne ne propose de carte correspondant à cette recherche.")
            )
        } else {
            grid
        }
    }

    private var grid: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 14) {
                if case let .player(username) = target {
                    playerHeader(username)
                }

                Text(CollectionCopy.cardCount(model.total))
                    .font(.caption)
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)

                LazyVGrid(columns: columns, spacing: 18) {
                    ForEach(model.cards, id: \.self) { card in
                        NavigationLink(value: CardDetailRoute(card: card)) {
                            CollectionCardCell(card: card)
                        }
                        .buttonStyle(.plain)
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

    /// No card count here: the route only carries the handle now, and the line right below
    /// already states how many tradable cards came back.
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
        SearchResultsView(route: SearchResultsRoute(target: .card(query: "Vampiric Tutor")))
    }
}
