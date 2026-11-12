import SwiftUI
import Testing
import UIKit

@testable import ArcaneCore

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
        // Read from the manifest: the test process has no app Info.plist to ask.
        let manifest = try String(contentsOfFile: Self.projectManifestPath, encoding: .utf8)
        #expect(manifest.contains("ASSETCATALOG_COMPILER_GLOBAL_ACCENT_COLOR_NAME: Primary"))
    }

    @Test func theAppsAccentIsTheSameColorSetAsPalettePrimary() throws {
        // The app compiles its own `Primary` color set for the system accent (an accent name
        // cannot point into the framework's bundle): the two copies must not drift.
        let appCopy = try String(
            contentsOfFile: Self.iosAppPath("ArcaneExchange/Assets.xcassets/Primary.colorset/Contents.json"),
            encoding: .utf8
        )
        let original = try String(
            contentsOfFile: Self.iosAppPath("ArcaneCore/Resources/Colors.xcassets/Primary.colorset/Contents.json"),
            encoding: .utf8
        )
        #expect(appCopy == original)
    }

    @Test func rarityTintsResolveToTheirAssets() {
        #expect(RarityColor.tint(forRarityCode: "m") == Color("RarityMythic", bundle: Palette.bundle))
        #expect(RarityColor.tint(forRarityCode: "S") == Color("RaritySpecial", bundle: Palette.bundle))
        #expect(RarityColor.tint(forRarityCode: "?") == .secondary)
    }

    private static let assetBundle = Palette.bundle

    /// A path under `ios-app/`, found from this file (the simulator reads the Mac's filesystem).
    private nonisolated static func iosAppPath(_ relative: String) -> String {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent(relative).path
    }

    private nonisolated static var projectManifestPath: String { iosAppPath("project.yml") }

    private static func hex(of name: String, in style: UIUserInterfaceStyle) throws -> UInt32 {
        let color = try #require(UIColor(named: name, in: Self.assetBundle, compatibleWith: nil))
        let resolved = color.resolvedColor(with: UITraitCollection(userInterfaceStyle: style))
        var red: CGFloat = 0, green: CGFloat = 0, blue: CGFloat = 0, alpha: CGFloat = 0
        #expect(resolved.getRed(&red, green: &green, blue: &blue, alpha: &alpha))
        let byte = { (component: CGFloat) in UInt32((component * 255).rounded()) }
        return byte(red) << 16 | byte(green) << 8 | byte(blue)
    }
}
