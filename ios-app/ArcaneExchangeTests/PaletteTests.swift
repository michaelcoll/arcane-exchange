import SwiftUI
import Testing
import UIKit

@testable import ArcaneExchange

/// The role colors are the web's `--primary*` / `--secondary*` / `--rarity-*` / `--down` tokens
/// (`frontend/app/assets/css/main.css`), converted from `oklch` to sRGB. Changing one
/// side without the other breaks this suite.
@MainActor
struct PaletteTests {
    /// Asset name → (light, dark), as `0xRRGGBB`.
    private nonisolated static let webValues: [String: (light: UInt32, dark: UInt32)] = [
        "Primary": (0x016C7A, 0x00DAF3),
        "PrimaryInk": (0x004A53, 0x74ECFC),
        "OnPrimary": (0xF5FEFF, 0x04181B),
        "Secondary": (0x7654BE, 0xCDBDFF),
        "SecondaryInk": (0x623BAB, 0xE0D7FF),
        "OnSecondary": (0xFCFAFF, 0x16121F),
        "RarityCommon": (0x333333, 0xA4A4A4),
        "RarityUncommon": (0x636C72, 0xAFB9C0),
        "RarityRare": (0x8C6200, 0xE0AF3B),
        "RarityMythic": (0xBC2D00, 0xF5642B),
        "RaritySpecial": (0xB43694, 0xEF6DC9),
        "Down": (0xC83B32, 0xDF8074),
    ]

    @Test(arguments: webValues.keys.sorted())
    func assetMatchesTheWebTokenInBothThemes(name: String) throws {
        let expected = try #require(Self.webValues[name])
        #expect(try Self.hex(of: name, in: .light) == expected.light)
        #expect(try Self.hex(of: name, in: .dark) == expected.dark)
    }

    @Test func theSystemAccentIsThePrimaryRole() throws {
        // `ASSETCATALOG_COMPILER_GLOBAL_ACCENT_COLOR_NAME` in `project.yml`: default tints,
        // prominent buttons and toggles must follow `Palette.primary`.
        let accent = try #require(Bundle.main.object(forInfoDictionaryKey: "NSAccentColorName") as? String)
        #expect(accent == "Primary")
    }

    @Test func rarityTintsResolveToTheirAssets() {
        #expect(RarityColor.tint(forRarityCode: "m") == Color("RarityMythic"))
        #expect(RarityColor.tint(forRarityCode: "S") == Color("RaritySpecial"))
        #expect(RarityColor.tint(forRarityCode: "?") == .secondary)
    }

    private static func hex(of name: String, in style: UIUserInterfaceStyle) throws -> UInt32 {
        let color = try #require(UIColor(named: name, in: .main, compatibleWith: nil))
        let resolved = color.resolvedColor(with: UITraitCollection(userInterfaceStyle: style))
        var red: CGFloat = 0, green: CGFloat = 0, blue: CGFloat = 0, alpha: CGFloat = 0
        #expect(resolved.getRed(&red, green: &green, blue: &blue, alpha: &alpha))
        let byte = { (component: CGFloat) in UInt32((component * 255).rounded()) }
        return byte(red) << 16 | byte(green) << 8 | byte(blue)
    }
}
