import SwiftUI

/// Set-icon tint by rarity, mirroring `frontend/`'s `RARITY_ICON_COLOR_CLASS`
/// (`app/utils/rarity.ts`) — same `--rarity-*` tokens, as asset-catalog colors here
/// since SwiftUI has no CSS custom properties to point at.
package enum RarityColor {
    package static func tint(forRarityCode code: String) -> Color {
        switch code.uppercased() {
        case "C": Color("RarityCommon", bundle: Palette.bundle)
        case "U": Color("RarityUncommon", bundle: Palette.bundle)
        case "R": Color("RarityRare", bundle: Palette.bundle)
        case "M": Color("RarityMythic", bundle: Palette.bundle)
        case "S": Color("RaritySpecial", bundle: Palette.bundle)
        default: .secondary
        }
    }
}
