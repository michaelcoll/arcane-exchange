import NukeUI
import SwiftUI

/// Pushed card screen (`ScrCard` in the iOS mockup): artwork, the price guide with its
/// 30-day chart, and — when the card is in the user's collection — their own copies.
/// Reused as-is by any card list; today only Collection pushes it.
struct CardDetailView: View {
    let card: CollectionCard

    @State private var model: CardDetailViewModel

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
                if let entry = card.collection_entry {
                    ownedSection(entry)
                }
                ownersLink
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
            .frame(maxWidth: .infinity)
        }
        .navigationTitle(card.name)
        .navigationBarTitleDisplayMode(.inline)
        .task { await model.loadHistory() }
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
        .frame(maxWidth: 230)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .shadow(color: .black.opacity(0.35), radius: 18, y: 12)
    }

    private var header: some View {
        VStack(spacing: 4) {
            Text(card.name)
                .font(.title2.weight(.semibold))
                .multilineTextAlignment(.center)
            Text(subtitle)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
        }
    }

    private var subtitle: String {
        var parts = [card.set_code.uppercased(), card.collector_number, card.language_code.uppercased()]
        if card.foil {
            parts.append("foil")
        }
        parts.append(RarityName.singular(card.rarity_code))
        return parts.joined(separator: " · ")
    }

    private var reservedBanner: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "lock.fill")
                .foregroundStyle(.purple)
            VStack(alignment: .leading, spacing: 2) {
                Text("Carte réservée")
                    .fontWeight(.semibold)
                Text("""
                Engagée dans un échange accepté. Elle ne peut pas être proposée ailleurs tant que \
                l'échange n'est pas clos ou abandonné.
                """)
                .font(.footnote)
                .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.purple.opacity(0.12), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: Price guide

    private var priceGuide: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 12) {
                metric("bas", cents: card.price_guide?.low.map(Int.init))
                metric("tendance", cents: card.price_guide?.trend.map(Int.init), accent: true)
                metric("moyenne", cents: card.price_guide?.avg.map(Int.init))
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

    private func metric(_ label: String, cents: Int?, accent: Bool = false) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption2)
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
            Text(cents.map { Price.euros(cents: $0) } ?? "—")
                .font(.callout.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(accent ? Color.accentColor : Color.primary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
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

        return VStack(alignment: .leading, spacing: 8) {
            Text("Dans ma collection")
                .font(.caption)
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
                .padding(.leading, 4)

            VStack(spacing: 0) {
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
            .padding(.horizontal, 14)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
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

    private var ownersLink: some View {
        NavigationLink(value: CardOffersRoute(card: card)) {
            Label(ownersTitle, systemImage: "person.2")
                .font(.subheadline.weight(.medium))
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
        }
        .buttonStyle(.bordered)
        .buttonBorderShape(.roundedRectangle(radius: 14))
        .tint(.secondary)
    }

    private var ownersTitle: String {
        if let count = card.owner_count, count > 0 {
            "Qui d'autre la possède (\(count))"
        } else {
            "Qui d'autre la possède"
        }
    }
}

#Preview {
    NavigationStack {
        CardDetailView(
            card: CollectionCard(
                collection_entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 980, quantity: 1),
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
