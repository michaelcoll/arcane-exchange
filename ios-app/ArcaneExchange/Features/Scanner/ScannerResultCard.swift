import ArcaneExchangeCore
import SwiftUI

/// The card the camera just recognised: its set symbol tinted by rarity, its name (in the language
/// of the card) and its references as printed on it, with the price of one copy on the right. Before
/// any, the hint to place a card, bare like the empty states of the open drawer.
struct ScannerResultCard: View {
    let card: RecognisedCard?
    /// The finish the card was read in.
    var foil = false
    /// What one copy is worth in that finish; nil shows a dash.
    var unitCents: Int?

    /// The height of the frame.
    private static let height: CGFloat = 76
    private static let shape = RoundedRectangle(cornerRadius: 16, style: .continuous)

    var body: some View {
        Group {
            if let card {
                text(of: card)
                    .background(.regularMaterial, in: Self.shape)
                    .frame(maxWidth: .infinity)
                    .frame(height: Self.height)
                    // Aligned on the buttons of the drawer.
                    .padding(.horizontal, 12)
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(card.accessibilityLabel(foil: foil, unitCents: unitCents))
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
            Text(Price.eurosOrDash(cents: unitCents))
                .font(.headline)
                .monospacedDigit()
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
                          foil: true, unitCents: 5000)
    }
    .padding()
}
