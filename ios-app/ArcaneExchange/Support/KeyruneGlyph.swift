import Foundation

/// Keyrune icon-font glyph lookup: Magic set code -> the private-use-area codepoint
/// that draws its symbol in the bundled `Keyrune.ttf` (mirrors `keyrune`'s `.ss-{code}`
/// CSS classes, which frontend-vue loads from jsdelivr instead of bundling the font).
///
/// The table itself lives in `KeyruneCodepointsA/B.swift`.
enum KeyruneGlyph {
    /// Generic "unknown set" glyph (`.ss:before` in keyrune.css), used when a set
    /// code has no entry in the table.
    static let fallback: Character = "\u{e684}"

    /// The glyph for `setCode` (case-insensitive), or `fallback` when unknown.
    static func glyph(forSetCode setCode: String) -> Character {
        guard let codepoint = KeyruneCodepoints.all[setCode.lowercased()],
              let scalar = Unicode.Scalar(codepoint)
        else { return fallback }
        return Character(scalar)
    }
}
