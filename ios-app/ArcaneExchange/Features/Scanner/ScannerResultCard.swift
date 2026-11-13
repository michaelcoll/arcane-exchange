import ArcaneExchangeCore
import NukeUI
import SwiftUI

/// The card the camera just recognised: the top of its image (its name frame and the start of its
/// art), or, while the image loads or when the platform has none, its name (French when it has
/// one) then its references as printed on the card. Before any, the hint to place a card, bare like
/// the empty states of the open drawer.
struct ScannerResultCard: View {
    let card: RecognisedCard?
    /// The finish the card was read in.
    var foil = false

    /// The height of the frame, image or text.
    private static let height: CGFloat = 76
    private static let shape = RoundedRectangle(cornerRadius: 16, style: .continuous)

    var body: some View {
        Group {
            if let card {
                LazyImage(url: CardArtwork.url(forSet: card.set, number: card.number)) { state in
                    if let image = state.image {
                        // Stretched to the width of the drawer buttons so the name reads, its top
                        // only, fading out at the bottom.
                        Color.clear
                            .overlay(alignment: .top) {
                                image.resizable().scaledToFill()
                            }
                            .clipped()
                            .mask {
                                LinearGradient(
                                    stops: [.init(color: .black, location: 0.45), .init(color: .clear, location: 1)],
                                    startPoint: .top,
                                    endPoint: .bottom
                                )
                            }
                    } else {
                        text(of: card)
                            .background(.regularMaterial, in: Self.shape)
                    }
                }
                // A new card starts from its own load, not from the image of the previous one.
                .id(card)
                .frame(maxWidth: .infinity)
                .frame(height: Self.height)
                // Aligned on the buttons of the drawer.
                .padding(.horizontal, 12)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("\(card.displayName), \(card.set) numéro \(card.number)\(foil ? ", foil" : "")")
            } else {
                VStack(spacing: 6) {
                    Image(systemName: "viewfinder")
                        .font(.title2)
                    Text("Place une carte dans le cadre")
                        .font(.subheadline)
                }
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
                .padding(.horizontal, 12)
            }
        }
        .animation(.snappy, value: card)
    }

    private func text(of card: RecognisedCard) -> some View {
        HStack(spacing: 12) {
            SetSymbol(setCode: card.set, size: 22)
                .foregroundStyle(RarityColor.tint(forRarityCode: card.rarity))
            VStack(alignment: .leading, spacing: 2) {
                Text(card.displayName)
                    .font(.headline)
                    .lineLimit(1)
                PrintedLine(number: card.number, set: card.set, foil: foil, language: card.language,
                            style: .subheadline)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }
}

#Preview {
    VStack {
        ScannerResultCard(card: nil)
        ScannerResultCard(card: RecognisedCard(set: "MOM", number: "123", language: .french, rarity: "R",
                                               name: "Elesh Norn", frenchName: "Elesh Norn, Mère des Machines"))
        ScannerResultCard(card: RecognisedCard(set: "MOM", number: "123", language: .french, rarity: "R",
                                               name: "Elesh Norn", frenchName: "Elesh Norn, Mère des Machines"),
                          foil: true)
    }
    .padding()
}

// A card of the local catalog with an image: it loads with the frontend running on localhost:3000.
#Preview("Avec image") {
    ScannerResultCard(card: RecognisedCard(set: "BLB", number: "155", language: .english, rarity: "R",
                                           name: "Sunspine Lynx", frenchName: nil))
        .padding(.vertical)
}
