import SwiftUI

/// Pushed card screen (`ScrCard` in the iOS mockup): artwork, the price guide with its
/// 30-day chart, and — when the card is in the user's collection — their own copies.
/// Reused as-is by any card list; today only Collection pushes it.
struct CardDetailView: View {
    let card: CollectionCard

    @State private var model: CardDetailViewModel
    @Environment(\.tradeNavigator) private var openTrade

    init(card: CollectionCard) {
        self.card = card
        _model = State(initialValue: CardDetailViewModel(card: card))
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                CardDetailArtwork(card: card)
                header
                if card.reserved {
                    CardReservedBanner()
                }
                CardPriceGuide(guide: card.price_guide, history: model.history)
                ownersSection
                if let entry = card.collection_entry {
                    CardOwnedCopies(entry: entry, trendCents: card.price_guide?.trend.map(Int.init))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
            .frame(maxWidth: .infinity)
        }
        .navigationBarTitleDisplayMode(.inline)
        .task { await model.loadHistory() }
        .task { await model.loadSetName() }
        .task { await model.loadOffers() }
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

    // MARK: Header

    private var header: some View {
        VStack(spacing: 4) {
            Text(card.name)
                .font(.title2.weight(.semibold))
                .multilineTextAlignment(.center)
            CardSetLine(card: card, setName: model.setName, isSetKnown: model.isSetKnown)
        }
    }

    // MARK: Owners

    /// The mockup lists the cheapest few offers right here, each with its own "Échanger"
    /// button, and only then links to the full list.
    ///
    /// The link stays put while the offers load or when they fail: it is the only way to the
    /// "Possesseurs" screen — which has its own error and retry — so a blip on this request
    /// must not strand the player. Only "nobody else has this card" hides the section, where
    /// an empty group would be noise between the chart and the collection figures.
    @ViewBuilder private var ownersSection: some View {
        switch model.offers {
        case let .loaded(preview, total) where total > 0:
            CardDetailGroup(header: CollectionCopy.offerCount(total)) {
                ForEach(Array(preview.enumerated()), id: \.element.owner_username) { index, offer in
                    if index > 0 {
                        Divider()
                    }
                    CardOfferRow(
                        offer: offer,
                        isStarting: model.startingWith == offer.owner_username,
                        // A reserved copy is locked into someone else's accepted trade.
                        action: offer.reserved ? nil : { start(offer) }
                    )
                }
                if !preview.isEmpty {
                    Divider()
                }
                ownersLink
            }
        case .loading, .failed:
            CardDetailGroup(header: "Possesseurs") { ownersLink }
        case .loaded:
            EmptyView()
        }
    }

    private var ownersLink: some View {
        NavigationLink(value: CardOffersRoute(card: card)) {
            HStack {
                Text("Voir tous les possesseurs")
                    .font(.subheadline)
                Spacer(minLength: 12)
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.tertiary)
            }
            .contentShape(Rectangle())
            .padding(.vertical, 13)
        }
        .buttonStyle(.plain)
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
        CardDetailView(
            card: CollectionCard(
                collection_entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 980, quantity: 1),
                collector_number: "243",
                foil: true,
                language_code: "fr",
                name: "The Soul Stone",
                price_guide: .init(avg: 870, low: 830, trend: 900),
                rarity_code: "M",
                reserved: true,
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "SOA"
            )
        )
    }
}
