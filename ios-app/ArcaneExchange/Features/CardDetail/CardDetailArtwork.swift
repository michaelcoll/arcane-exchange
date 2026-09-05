import NukeUI
import SwiftUI

/// The card screen's hero: the artwork at full width, lifted off the page by its halo.
struct CardDetailArtwork: View {
    let card: CollectionCard

    private static let cornerRadius: CGFloat = 12

    var body: some View {
        let url = CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)
        LazyImage(url: url) { state in
            if let image = state.image {
                image.resizable().scaledToFit()
            } else {
                placeholder(hasFailed: state.error != nil)
            }
        }
        .aspectRatio(5.0 / 7.0, contentMode: .fit)
        .frame(maxWidth: 320)
        .clipShape(RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous))
        .cardArtworkLift(
            tint: nil,
            shadowOpacity: 0.35,
            shadowRadius: 6,
            shadowY: 6,
            haloRadius: 6
        )
    }

    private func placeholder(hasFailed: Bool) -> some View {
        RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous)
            .fill(.quaternary)
            .overlay {
                if hasFailed {
                    Text(card.name)
                        .font(.caption)
                        .multilineTextAlignment(.center)
                        .foregroundStyle(.secondary)
                        .padding(8)
                } else {
                    ProgressView()
                }
            }
    }
}

#Preview {
    CardDetailArtwork(
        card: CollectionCard(
            collector_number: "243",
            foil: true,
            language_code: "fr",
            name: "The Soul Stone",
            price_guide: .init(avg: 870, low: 830, trend: 900),
            rarity_code: "M",
            reserved: false,
            scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
            set_code: "SOA"
        )
    )
    .padding(40)
}
