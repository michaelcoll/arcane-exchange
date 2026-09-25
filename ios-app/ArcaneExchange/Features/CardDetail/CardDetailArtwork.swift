import SwiftUI

/// The card screen's hero: the artwork at full width, lifted off the page by its halo. A
/// double-faced card turns over to its back with the button in its corner.
struct CardDetailArtwork: View {
    let card: CollectionCard

    /// Which face is shown; switches halfway through the turn, when the card is edge-on.
    @State private var showsBack = false
    /// Whether the card is turned over, animated over the whole turn.
    @State private var isTurned = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private static let cornerRadius: CGFloat = 12

    var body: some View {
        faces
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .frame(maxWidth: 320)
            .cardArtworkLift(
                tint: nil,
                shadowOpacity: 0.35,
                shadowRadius: 6,
                shadowY: 6,
                haloRadius: 6
            )
            .overlay(alignment: .bottomTrailing) { flipButton }
    }

    /// Both faces stacked, the back pre-turned: rotating the stack half a turn shows it the
    /// right way round. Each face is hidden once it faces away, halfway through the turn. With
    /// Reduce Motion, nothing turns: the faces just cross-fade.
    @ViewBuilder private var faces: some View {
        let turns = !reduceMotion
        ZStack {
            face(card.image_url)
                .opacity(showsBack ? 0 : 1)
            if card.image_back_url != nil {
                face(card.image_back_url)
                    .rotation3DEffect(.degrees(turns ? 180 : 0), axis: (x: 0, y: 1, z: 0))
                    .opacity(showsBack ? 1 : 0)
            }
        }
        .rotation3DEffect(.degrees(turns && isTurned ? 180 : 0), axis: (x: 0, y: 1, z: 0), perspective: 0.4)
    }

    private func face(_ path: String?) -> some View {
        CardImageView(url: CardArtwork.url(imagePath: path))
            .clipShape(RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous))
    }

    @ViewBuilder private var flipButton: some View {
        if card.image_back_url != nil {
            Button {
                flip()
            } label: {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .font(.body.weight(.semibold))
                    .foregroundStyle(.white)
                    .frame(width: 40, height: 40)
                    .background(.black.opacity(0.55), in: Circle())
                    .background(.ultraThinMaterial, in: Circle())
            }
            .buttonStyle(.plain)
            .padding(10)
            .accessibilityLabel(showsBack ? "Voir le recto" : "Voir le verso")
        }
    }

    /// An ease-in-out half turn is edge-on at exactly half its duration: the faces swap then.
    private func flip() {
        guard !reduceMotion else {
            withAnimation(.easeInOut(duration: 0.2)) { showsBack.toggle() }
            return
        }
        let duration = 0.5
        withAnimation(.easeInOut(duration: duration)) { isTurned.toggle() }
        withAnimation(.linear(duration: 0.01).delay(duration / 2)) { showsBack.toggle() }
    }
}

#Preview {
    CardDetailArtwork(
        card: CollectionCard(
            collector_number: "243",
            foil: true,
            image_back_url: "/card-images/SOA_243_EN_back.webp?v=gatherer",
            image_url: "/card-images/SOA_243_EN.webp?v=gatherer",
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
