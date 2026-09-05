import SwiftUI

/// Set-icon tint by rarity, mirroring `frontend-vue`'s `RARITY_ICON_COLOR_CLASS`
/// (`app/utils/rarity.ts`) — same `--rarity-*` tokens, as asset-catalog colors here
/// since SwiftUI has no CSS custom properties to point at.
enum RarityColor {
    static func tint(forRarityCode code: String) -> Color {
        switch code.uppercased() {
        case "C": Color("RarityCommon")
        case "U": Color("RarityUncommon")
        case "R": Color("RarityRare")
        case "M": Color("RarityMythic")
        case "S": Color("RaritySpecial")
        default: .secondary
        }
    }
}
