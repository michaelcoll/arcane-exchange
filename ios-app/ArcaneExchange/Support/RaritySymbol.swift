import SwiftUI

/// A rarity drawn the way Magic itself shows one: a set symbol tinted by rarity — here the
/// Foundation (FDN) symbol, a neutral modern set that reads well in all five tints.
struct RaritySymbol: View {
    let rarity: RarityCode
    var size: CGFloat = 17

    /// The set whose symbol stands in for "a rarity" outside of any particular card.
    private static let setCode = "FDN"

    var body: some View {
        SetSymbol(setCode: Self.setCode, size: size)
            .foregroundStyle(RarityColor.tint(forRarityCode: rarity.rawValue))
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 10) {
        ForEach(RarityCode.allCases, id: \.self) { rarity in
            HStack(spacing: 12) {
                RaritySymbol(rarity: rarity)
                Text(rarity.label)
            }
        }
    }
    .padding()
}
