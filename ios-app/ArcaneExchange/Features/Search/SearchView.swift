import APIClient
import ArcaneExchangeCore
import SwiftUI

/// Rechercher tab (`ScrSearch` in the iOS mockup): a mode switch — carte / joueur —
/// over the system search field. Recent card queries and players are kept on the device only,
/// as the mockup notes.
///
/// The card mode works like the Photos app: the full card grid follows the field as it is
/// typed, and submitting only puts the keyboard away, the grid stays where it is. Its axes,
/// sort and filters sit in the "…" menu of the nav bar.
struct SearchView: View {
    @Binding var searchText: String

    @State private var model: SearchViewModel
    @State private var cardResults: SearchResultsViewModel
    @State private var mode: SearchMode = .card
    @Namespace private var modeThumb
    @Environment(AppRouter.self) private var router
    private let api: any APIProtocol

    init(searchText: Binding<String>, api: any APIProtocol) {
        _searchText = searchText
        self.api = api
        _model = State(initialValue: SearchViewModel(api: api))
        _cardResults = State(initialValue: SearchResultsViewModel(target: .card(query: ""), api: api))
    }

    /// Heterogeneous on purpose: the stack pushes `SearchResultsRoute`, then from the results
    /// grid a `CardDetailRoute`, then a `CardOffersRoute`. A typed `[SearchResultsRoute]` path
    /// would silently swallow the latter two.
    @State private var path = NavigationPath()

    var body: some View {
        NavigationStack(path: $path) {
            VStack(spacing: 0) {
                modePicker
                content
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .animation(.snappy, value: mode)
            }
            // The picker band would otherwise sit on the plain `systemBackground` while the
            // Form/List below is on `systemGroupedBackground` — one seamless colour instead.
            .background(Color(.systemGroupedBackground))
            .navigationTitle("Rechercher")
            .navigationBarTitleDisplayMode(.inline)
            .accountToolbar()
            .toolbar {
                if mode == .card {
                    ToolbarItem(placement: .topBarTrailing) {
                        SearchOptionsMenu(model: cardResults)
                    }
                }
            }
            .cardBrowsingDestinations()
            .tradeDestinations()
        }
        .tradeNavigation(path: $path)
        .searchable(text: $searchText, prompt: mode.prompt)
        // Search is active as soon as the tab opens (`.searchTabSelection`): without this, the
        // system hides the nav bar, and the "…" menu with it, for as long as it stays active.
        .searchPresentationToolbarBehavior(.avoidHidingContent)
        .onSubmit(of: .search) { submit() }
        .onChange(of: mode) { searchText = "" }
        // An impression opened by a link (Siri, Spotlight, Shortcuts) replaces whatever the
        // stack showed. The router holds it until this tab is on screen, so a cold start works.
        .onChange(of: router.pendingImpression, initial: true) { _, pending in
            guard let pending else { return }
            router.pendingImpression = nil
            Task { await open(pending) }
        }
        .task(id: lookupKey) {
            switch mode {
            case .card: await cardResults.search(searchText)
            case .player: await model.lookUpPlayers(matching: searchText)
            }
        }
    }

    /// Re-runs the debounced live preview whenever the mode or the field text changes.
    private var lookupKey: String {
        "\(mode.rawValue)|\(searchText)"
    }

    // MARK: Mode switch

    private var modePicker: some View {
        Picker("Type de recherche", selection: $mode) {
            ForEach(SearchMode.allCases) { Text($0.label).tag($0) }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    @ViewBuilder private var content: some View {
        switch mode {
        case .card: cardMode
        case .player: playerMode
        }
    }

    // MARK: Carte

    /// A blank field shows the recent searches; as soon as it holds text, the live grid.
    @ViewBuilder private var cardMode: some View {
        let typing = !searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty

        if typing {
            SearchResultsGrid(model: cardResults, showsInlineControls: false)
        } else if model.recentQueries.isEmpty {
            ContentUnavailableView {
                Label("Rechercher une carte", systemImage: "magnifyingglass")
            } description: {
                Text("Tape un nom de carte dans la barre de recherche.")
            }
        } else {
            List {
                Section {
                    ForEach(model.recentQueries, id: \.self) { query in
                        Button {
                            searchText = query
                            submit()
                        } label: {
                            Label(query, systemImage: "clock.arrow.circlepath")
                        }
                        .tint(.primary)
                    }
                    .onDelete { offsets in
                        model.removeQueries(Set(offsets.map { model.recentQueries[$0] }))
                    }
                } header: {
                    HStack {
                        Text("Recherches récentes")
                        Spacer()
                        Button("Effacer") { model.clearQueries() }
                            .font(.caption)
                            .textCase(nil)
                    }
                }
            }
        }
    }

    // MARK: Joueur

    @ViewBuilder private var playerMode: some View {
        let typing = !searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty

        if !typing, model.recentPlayers.isEmpty {
            ContentUnavailableView {
                Label("Chercher un joueur", systemImage: "person.fill.viewfinder")
            } description: {
                Text("""
                Saisis un pseudo dans le champ de recherche pour trouver un joueur qui propose \
                des cartes à l'échange. Seuls les joueurs avec au moins une carte échangeable \
                apparaissent.
                """)
            }
        } else {
            List {
                if typing {
                    Section {
                        if model.isLoadingPlayers, model.playerSuggestions.isEmpty {
                            HStack { ProgressView(); Text("Recherche…").foregroundStyle(.secondary) }
                        } else if model.playerLookupFailed {
                            Label("Recherche indisponible", systemImage: "exclamationmark.triangle")
                                .foregroundStyle(.secondary)
                        } else if model.playerSuggestions.isEmpty {
                            Text("Aucun joueur. Un joueur privé, ou qui ne propose rien, n'apparaît jamais ici.")
                                .foregroundStyle(.secondary)
                        } else {
                            ForEach(model.playerSuggestions, id: \.username) { playerRow($0) }
                        }
                    } header: {
                        Text("Joueurs qui proposent des cartes")
                    }
                } else {
                    Section {
                        ForEach(model.recentPlayers, id: \.username) { playerRow($0) }
                    } header: {
                        Text("Joueurs récents")
                    }
                }
            }
        }
    }

    private func playerRow(_ user: UserSuggestion) -> some View {
        Button {
            model.rememberPlayer(user)
            path.append(SearchResultsRoute(target: .player(username: user.username)))
        } label: {
            HStack(spacing: 12) {
                PlayerAvatar(username: user.username)
                VStack(alignment: .leading, spacing: 2) {
                    UsernameLabel(username: user.username)
                        .fontWeight(.medium)
                    Text(user.tradableCountLabel)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                Image(systemName: "chevron.right")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
        .tint(.primary)
    }

    // MARK: Link

    /// Looks the impression up again by set and number, then shows its page. An impression that
    /// is not found, or a failed call, leaves the tab as it was.
    private func open(_ pending: AppRouter.PendingImpression) async {
        guard let card = await ImpressionLinkResolver(api: api).card(for: pending) else { return }
        switch pending {
        case .detail:
            path = NavigationPath([CardDetailRoute(card: card.asCollectionCard)])
        }
    }

    // MARK: Submit

    /// For a card search, the grid already shows the results: submitting only remembers the
    /// query, and the system puts the keyboard away.
    private func submit() {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return }
        switch mode {
        case .card:
            model.rememberQuery(query)
        case .player:
            // Enter with no picked suggestion: jump straight into the top match if there is one.
            if let first = model.playerSuggestions.first {
                model.rememberPlayer(first)
                path.append(SearchResultsRoute(target: .player(username: first.username)))
            }
        }
    }
}

#Preview {
    SearchView(searchText: .constant(""), api: APIClientFactory.preview)
        .environment(AppRouter())
}
