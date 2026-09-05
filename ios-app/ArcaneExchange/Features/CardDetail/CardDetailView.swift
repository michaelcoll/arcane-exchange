import NukeUI
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
                artwork
                header
                if card.reserved {
                    reservedBanner
                }
                priceGuide
                ownersSection
                if let entry = card.collection_entry {
                    ownedSection(entry)
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

    // MARK: Artwork & header

    private var artwork: some View {
        let url = CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)
        return LazyImage(url: url) { state in
            if let image = state.image {
                image.resizable().scaledToFit()
            } else {
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(.quaternary)
                    .overlay {
                        if state.error == nil {
                            ProgressView()
                        } else {
                            Text(card.name)
                                .font(.caption)
                                .multilineTextAlignment(.center)
                                .foregroundStyle(.secondary)
                                .padding(8)
                        }
                    }
            }
        }
        .aspectRatio(5.0 / 7.0, contentMode: .fit)
        .frame(maxWidth: 320)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .shadow(color: .black.opacity(0.35), radius: 18, y: 12)
    }

    private var header: some View {
        VStack(spacing: 4) {
            Text(card.name)
                .font(.title2.weight(.semibold))
                .multilineTextAlignment(.center)
            CardSetLine(card: card, setName: model.setName, isSetKnown: model.isSetKnown)
        }
    }

    private var reservedBanner: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "lock.fill")
                .foregroundStyle(.purple)
            VStack(alignment: .leading, spacing: 2) {
                Text("Carte réservée")
                    .fontWeight(.semibold)
                Text(
                    """
                    Engagée dans un échange accepté. Elle ne peut pas être proposée ailleurs tant que \
                    l'échange n'est pas clos ou abandonné.
                    """
                )
                .font(.footnote)
                .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.purple.opacity(0.12), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: Price guide

    /// Trend headlined on the left, low and average as a small aside on the right — the
    /// mockup's `pxhead`, which gives the number people actually read the most room.
    private var priceGuide: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .center, spacing: 12) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("tendance")
                        .font(.caption2.monospaced())
                        .textCase(.uppercase)
                        .foregroundStyle(.secondary)
                    Text(price(card.price_guide?.trend))
                        .font(.system(size: 30, weight: .semibold).monospacedDigit())
                }
                Spacer(minLength: 0)
                VStack(alignment: .trailing, spacing: 3) {
                    aside("bas", cents: card.price_guide?.low)
                    aside("moyenne", cents: card.price_guide?.avg)
                }
            }
            chartArea
                .frame(height: 150)
                .animation(.snappy, value: model.history)
            Text("30 derniers jours · Cardmarket")
                .font(.caption2.monospaced())
                .foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func aside(_ label: String, cents: Int32?) -> some View {
        HStack(spacing: 5) {
            Text(label)
                .foregroundStyle(.secondary)
            Text(price(cents))
                .fontWeight(.semibold)
        }
        .font(.caption.monospaced())
    }

    private func price(_ cents: Int32?) -> String {
        cents.map { Price.euros(cents: Int($0)) } ?? "—"
    }

    @ViewBuilder private var chartArea: some View {
        switch model.history {
        case .loading:
            chartPlaceholder { ProgressView() }
        case let .ready(points):
            PriceHistoryChart(points: points)
                .transition(.opacity)
        case .notEnoughData:
            chartPlaceholder { Text("Pas encore assez d'historique") }
        case .failed:
            chartPlaceholder { Text("Historique indisponible") }
        }
    }

    private func chartPlaceholder(@ViewBuilder _ content: () -> some View) -> some View {
        content()
            .font(.caption2.monospaced())
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: Owned copies

    private func ownedSection(_ entry: CollectionEntry) -> some View {
        let purchase = Int(entry.purchase_price)
        let trendCents = card.price_guide?.trend.map(Int.init)
        let deal = CardDeal(purchaseCents: purchase, trendCents: trendCents)

        return group("Dans ma collection") {
            detailRow("Exemplaires") { Text("×\(entry.quantity)") }
            Divider()
            detailRow("Prix d'achat") { Text(Price.euros(cents: purchase)) }
            if let deal, let trendCents {
                Divider()
                detailRow("Écart depuis l'achat") {
                    Text(spread(from: purchase, to: trendCents, deal: deal))
                        .foregroundStyle(deal.kind == .bad ? Color.red : Color.accentColor)
                }
            }
        }
    }

    private func detailRow(_ title: String, @ViewBuilder trailing: () -> some View) -> some View {
        HStack {
            Text(title)
            Spacer(minLength: 12)
            trailing()
                .fontWeight(.medium)
                .monospacedDigit()
        }
        .font(.subheadline)
        .padding(.vertical, 12)
    }

    private func spread(from purchase: Int, to trend: Int, deal: CardDeal) -> String {
        let delta = trend - purchase
        let sign = delta >= 0 ? "+" : "−"
        return "\(sign)\(Price.euros(cents: abs(delta))) · \(abs(deal.percent)) %"
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
            group(CollectionCopy.offerCount(total)) {
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
            group("Possesseurs") { ownersLink }
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

    // MARK: Grouped list

    /// The mockup's `IGroup`: an uppercase caption over a rounded card of rows.
    private func group(_ header: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(header)
                .font(.caption)
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
                .padding(.leading, 4)

            VStack(spacing: 0) { content() }
                .padding(.horizontal, 14)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
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
                rarity_code: "R",
                reserved: false,
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "SOA"
            )
        )
    }
}
