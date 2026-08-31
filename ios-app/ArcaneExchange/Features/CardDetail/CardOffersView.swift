import NukeUI
import SwiftUI

/// "Possesseurs" (`ScrOffers` in the mockup): the other players offering this card to trade,
/// cheapest first. Read-only for now — rows do not start a trade.
struct CardOffersView: View {
    let card: CollectionCard

    @State private var model: CardOffersViewModel

    init(card: CollectionCard) {
        self.card = card
        _model = State(initialValue: CardOffersViewModel(card: card))
    }

    var body: some View {
        content
            .navigationTitle("Possesseurs")
            .navigationBarTitleDisplayMode(.inline)
            .task { await model.load() }
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
            List {
                Section { cardRow }

                Section {
                    ForEach(offers, id: \.owner_username) { OfferRow(offer: $0) }
                } header: {
                    Text(CollectionCopy.offerCount(offers.count))
                } footer: {
                    Text("""
                    Une copie réservée est engagée dans un échange déjà accepté : elle reste \
                    visible mais ne peut pas être demandée.
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

            VStack(alignment: .leading, spacing: 2) {
                Text(card.name)
                    .fontWeight(.semibold)
                    .lineLimit(1)
                if let trend = card.price_guide?.trend {
                    Text("tendance \(Price.euros(cents: Int(trend)))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }
}

/// One player's offer: avatar, handle, quantity, and either a price or a reserved lock.
private struct OfferRow: View {
    let offer: CardOffer

    var body: some View {
        HStack(spacing: 12) {
            PlayerAvatar(username: offer.owner_username)
            VStack(alignment: .leading, spacing: 2) {
                Text("@\(offer.owner_username)")
                    .fontWeight(.medium)
                Text("×\(offer.quantity) dispo")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 8)
            if offer.reserved {
                Label("Réservée", systemImage: "lock.fill")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.purple)
            } else if let price = offer.selling_price {
                Text(Price.euros(cents: Int(price)))
                    .font(.callout.weight(.semibold))
                    .monospacedDigit()
                    .foregroundStyle(Color.accentColor)
            }
        }
        .padding(.vertical, 4)
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
