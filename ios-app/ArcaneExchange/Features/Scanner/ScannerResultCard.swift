import ArcaneExchangeCore
import SwiftUI

/// The card the camera just recognised: its name (French when it has one), then its set and number.
struct ScannerResultCard: View {
    let card: RecognisedCard?

    var body: some View {
        HStack(spacing: 12) {
            if let card {
                SetSymbol(setCode: card.set, size: 22)
                    .foregroundStyle(RarityColor.tint(forRarityCode: card.rarity))
                VStack(alignment: .leading, spacing: 2) {
                    Text(card.displayName)
                        .font(.headline)
                        .lineLimit(2)
                    Text("\(card.set) · #\(card.number)")
                        .font(.subheadline.monospaced())
                        .foregroundStyle(.secondary)
                }
            } else {
                Image(systemName: "viewfinder")
                    .font(.title3)
                    .foregroundStyle(.secondary)
                Text("Place une carte dans le cadre")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 0)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .animation(.snappy, value: card)
    }
}

#Preview {
    VStack {
        ScannerResultCard(card: nil)
        ScannerResultCard(card: RecognisedCard(set: "MOM", number: "123", language: .french, rarity: "R",
                                               name: "Elesh Norn", frenchName: "Elesh Norn, Mère des Machines"))
    }
    .padding()
}
