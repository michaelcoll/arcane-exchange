import SwiftUI

/// A set's Keyrune symbol on its own, at a fixed width so the labels next to it line up.
/// Uncoloured on purpose: the caller tints it (by rarity on a card, neutrally in a filter).
struct SetSymbol: View {
    let setCode: String
    var size: CGFloat = 17

    var body: some View {
        Text(String(KeyruneGlyph.glyph(forSetCode: setCode)))
            .font(.custom("Keyrune", size: size))
            .frame(width: size + 6, alignment: .center)
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 10) {
        SetSymbol(setCode: "MH3")
        SetSymbol(setCode: "FDN")
        // Unknown code: falls back to Keyrune's generic "unknown set" glyph.
        SetSymbol(setCode: "ZZZ")
    }
    .padding()
}
