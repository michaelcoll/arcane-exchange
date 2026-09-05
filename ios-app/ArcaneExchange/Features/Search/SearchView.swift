import NukeUI
import SwiftUI

/// Rechercher tab (`ScrSearch` in the iOS mockup): a mode switch — carte / decklist / joueur —
/// over the system search field. Card and player search hit the real API; decklist has no
/// endpoint yet and lands on a placeholder. Recent card queries and players are kept on the
/// device only, as the mockup notes.
struct SearchView: View {
    @Binding var searchText: String

    @State private var model = SearchViewModel()
    @State private var mode: SearchMode = .card
    @State private var deckList = ""
    @Namespace private var modeThumb

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
            .cardBrowsingDestinations()
            .tradeDestinations()
        }
        .tradeNavigation(path: $path)
        .searchable(text: $searchText, prompt: mode.prompt)
        .onSubmit(of: .search) { submit() }
        .onChange(of: mode) { searchText = "" }
        .task(id: lookupKey) {
            switch mode {
            case .card: await model.previewCards(matching: searchText)
            case .player: await model.lookUpPlayers(matching: searchText)
            case .decklist: break
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
        case .decklist: decklistMode
        case .player: playerMode
        }
    }

    // MARK: Carte

    @ViewBuilder private var cardMode: some View {
        let typing = !searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty

        if !typing, model.recentQueries.isEmpty {
            ContentUnavailableView {
                Label("Rechercher une carte", systemImage: "magnifyingglass")
            } description: {
                Text("Tape un nom de carte dans la barre de recherche.")
            }
        } else {
            List {
                if !filteredRecents.isEmpty {
                    Section {
                        ForEach(filteredRecents, id: \.self) { query in
                            Button {
                                searchText = query
                                submit()
                            } label: {
                                Label(query, systemImage: "clock.arrow.circlepath")
                            }
                            .tint(.primary)
                        }
                        .onDelete { offsets in
                            model.removeQueries(Set(offsets.map { filteredRecents[$0] }))
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

                if typing {
                    Section {
                        cardPreviewContent
                    } header: {
                        Text("Résultats")
                    }
                }
            }
        }
    }

    @ViewBuilder private var cardPreviewContent: some View {
        if model.isLoadingCardPreview, model.cardPreview.isEmpty {
            HStack {
                ProgressView()
                Text("Recherche…").foregroundStyle(.secondary)
            }
        } else if model.cardPreviewFailed {
            Label("Recherche indisponible", systemImage: "exclamationmark.triangle")
                .foregroundStyle(.secondary)
        } else if model.cardPreview.isEmpty {
            Text("Aucune carte ne correspond.")
                .foregroundStyle(.secondary)
        } else {
            ForEach(model.cardPreview, id: \.self) { card in
                NavigationLink(value: CardDetailRoute(card: card)) {
                    CardPreviewRow(card: card)
                }
            }
            Button { submit() } label: {
                Label("Voir tous les résultats", systemImage: "arrow.right")
            }
        }
    }

    private var filteredRecents: [String] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return query.isEmpty ? model.recentQueries : model.recentQueries.filter { $0.lowercased().contains(query) }
    }

    // MARK: Decklist

    private var decklistMode: some View {
        Form {
            Section {
                TextEditor(text: $deckList)
                    .font(.callout.monospaced())
                    .frame(minHeight: 160)
                    .overlay(alignment: .topLeading) {
                        if deckList.isEmpty {
                            Text("1x Vampiric Tutor\n1x Black Market Connections\n1x The Soul Stone…")
                                .font(.callout.monospaced())
                                .foregroundStyle(.tertiary)
                                .padding(.top, 8)
                                .allowsHitTesting(false)
                        }
                    }
            } header: {
                Text("Coller une decklist")
            } footer: {
                Text("Moxfield, Archidekt ou texte brut, une carte par ligne.")
            }

            Section {
                Button("Trouver les joueurs") {
                    path.append(SearchResultsRoute(target: .decklist))
                }
                .frame(maxWidth: .infinity)
                .disabled(deckList.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
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

    // MARK: Submit

    private func submit() {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return }
        switch mode {
        case .card, .decklist:
            model.rememberQuery(query)
            path.append(SearchResultsRoute(target: .card(query: query)))
        case .player:
            // Enter with no picked suggestion: jump straight into the top match if there is one.
            if let first = model.playerSuggestions.first {
                model.rememberPlayer(first)
                path.append(SearchResultsRoute(target: .player(username: first.username)))
            }
        }
    }
}

/// Compact card row for the as-you-type preview list — thumbnail, name, set, trend price.
private struct CardPreviewRow: View {
    let card: CollectionCard

    var body: some View {
        HStack(spacing: 12) {
            LazyImage(url: CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)) { state in
                if let image = state.image {
                    image.resizable().scaledToFill()
                } else {
                    Rectangle().fill(.quaternary)
                }
            }
            .frame(width: 34, height: 47)
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))

            VStack(alignment: .leading, spacing: 2) {
                Text(card.name)
                    .fontWeight(.medium)
                    .lineLimit(1)
                Text("\(card.set_code.uppercased()) · \(card.collector_number)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer(minLength: 8)

            if let trend = card.price_guide?.trend {
                Text(Price.euros(cents: Int(trend)))
                    .font(.callout.weight(.semibold))
                    .monospacedDigit()
            }
        }
        .padding(.vertical, 2)
    }
}

#Preview {
    SearchView(searchText: .constant(""))
}
