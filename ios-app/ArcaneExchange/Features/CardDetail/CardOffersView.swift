import NukeUI
import SwiftUI

/// "Possesseurs" (`ScrOffers` in the mockup): the other players offering this card to trade,
/// cheapest first, reserved copies last. Tapping a row — the whole row, see `CardOfferRow` —
/// opens the trade with that player, this card already on the table. A reserved row is inert.
struct CardOffersView: View {
    let card: CollectionCard

    @State private var model: CardOffersViewModel
    @Environment(\.tradeNavigator) private var openTrade

    init(card: CollectionCard) {
        self.card = card
        _model = State(initialValue: CardOffersViewModel(card: card))
    }

    var body: some View {
        content
            .navigationTitle("Possesseurs")
            .navigationBarTitleDisplayMode(.inline)
            .task { await model.load() }
            .task { await model.loadSetName() }
            .alert(
                "Échange impossible",
                isPresented: Binding(get: { model.startError != nil }, set: {
                    if !$0 {
                        model.startError = nil
                    }
                }),
                actions: { Button("OK", role: .cancel) { model.startError = nil } },
                message: { Text(model.startError ?? "") }
            )
    }

    @ViewBuilder private var content: some View {
        switch model.state {
        case .loading:
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)

        case .failed:
            ContentUnavailableView(
                label: { Label("Offres indisponibles", systemImage: "person.2.slash") },
                description: { Text("Impossible de charger les possesseurs pour l'instant.") },
                actions: { Button("Réessayer") { Task { await model.load() } } }
            )

        case let .loaded(offers) where offers.isEmpty:
            ContentUnavailableView(
                "Personne pour l'instant",
                systemImage: "person.2",
                description: Text("Personne d'autre ne possède cette carte pour l'instant.")
            )

        case let .loaded(offers):
            // Reserved copies sink to the bottom: they are context, not something to act on.
            let available = offers.filter { !$0.reserved }
            let reserved = offers.filter(\.reserved)
            List {
                Section { cardRow }

                Section {
                    ForEach(available + reserved, id: \.owner_username) { offer in
                        CardOfferRow(
                            offer: offer,
                            isStarting: model.startingWith == offer.owner_username,
                            // A reserved copy is locked into someone else's accepted trade.
                            action: offer.reserved ? nil : { start(offer) }
                        )
                    }
                } header: {
                    Text(CollectionCopy.offerAvailability(available: available.count, reserved: reserved.count))
                } footer: {
                    Text("""
                    Une copie réservée est engagée dans un échange déjà accepté : elle reste \
                    visible mais ne peut pas être demandée. Ouvrir un échange avec un joueur \
                    avec qui tu en as déjà un ajoute simplement la carte à celui-ci.
                    """)
                }
            }
        }
    }

    private var cardRow: some View {
        HStack(spacing: 12) {
            let url = CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)
            LazyImage(url: url) { state in
                if let image = state.image {
                    image.resizable().scaledToFill()
                } else {
                    Rectangle().fill(.quaternary)
                }
            }
            .frame(width: 40, height: 56)
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))

            VStack(alignment: .leading, spacing: 3) {
                Text(card.name)
                    .fontWeight(.semibold)
                    .lineLimit(1)
                CardSetLine(card: card, setName: model.setName, isSetKnown: model.isSetKnown, size: 11.5)
            }
        }
    }

    private func start(_ offer: CardOffer) {
        Task {
            if let route = await model.startTrade(with: offer) {
                openTrade(route)
            }
        }
    }
}

#Preview {
    NavigationStack {
        CardOffersView(
            card: CollectionCard(
                collector_number: "243",
                foil: false,
                language_code: "fr",
                name: "The Soul Stone",
                price_guide: .init(avg: 870, low: 830, trend: 900),
                rarity_code: "R",
                reserved: false,
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "FIN"
            )
        )
    }
}
