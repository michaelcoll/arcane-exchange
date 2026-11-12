import SwiftUI

/// The design system's two accents, named by role like the web's `--primary*` / `--secondary*`
/// tokens (`frontend/app/assets/css/main.css`) — same light and dark values, pinned by
/// `PaletteTests`. Pick one by what it means, never by its look:
///
/// - **primary**: my actions, rising values, a trade's progress;
/// - **secondary**: another player or the exchange — reserved cards, what I receive, the partner.
///
/// A namespace rather than `ShapeStyle` members: SwiftUI's `.primary` / `.secondary` are the
/// label hierarchy, and a same-named accent would silently shadow or be shadowed by them.
package enum Palette {
    /// Where the color sets live: this framework, not the process's main bundle (the app, the
    /// widget extension or a test runner).
    package static let bundle = Bundle(for: BundleToken.self)

    private final class BundleToken {}

    /// Also the system accent (`ASSETCATALOG_COMPILER_GLOBAL_ACCENT_COLOR_NAME`, fed by a copy of
    /// this color set in the app's own catalog, kept identical by `PaletteTests`), so default
    /// tints, prominent buttons and toggles already use it.
    package static var primary: Color {
        Color("Primary", bundle: Palette.bundle)
    }

    /// `--primary-ink`: text and glyphs sitting on a primary-tinted fill.
    package static var primaryInk: Color {
        Color("PrimaryInk", bundle: Palette.bundle)
    }

    /// `--on-primary`: text and glyphs on a solid primary background.
    package static var onPrimary: Color {
        Color("OnPrimary", bundle: Palette.bundle)
    }

    package static var secondary: Color {
        Color("Secondary", bundle: Palette.bundle)
    }

    /// `--secondary-ink`: text and glyphs sitting on a secondary-tinted fill.
    package static var secondaryInk: Color {
        Color("SecondaryInk", bundle: Palette.bundle)
    }

    /// `--on-secondary`: text and glyphs on a solid secondary background.
    package static var onSecondary: Color {
        Color("OnSecondary", bundle: Palette.bundle)
    }

    /// `--down`: a value going down — a falling collection, a loss since purchase. Rising values are
    /// `primary`.
    package static var down: Color {
        Color("Down", bundle: Palette.bundle)
    }
}

package extension View {
    /// The mockup's tinted accent box: a secondary fill always carries a secondary line.
    func tintSecondary(in shape: some InsettableShape) -> some View {
        background(Palette.secondary.opacity(0.12), in: shape)
            .overlay { shape.strokeBorder(Palette.secondary.opacity(0.4)) }
    }

    /// The mockup's `.reserved-flag` chip (`styles.css`): an opaque secondary-tinted surface so
    /// the badge never has to fight the artwork behind it, `--secondary-ink` text, and a
    /// `--secondary-line` border.
    func reservedBadgeChip(in shape: some InsettableShape) -> some View {
        // The tint layer sits on an opaque surface, not on the artwork: `.background`
        // stacks backwards, so the tint is applied first and the surface behind it.
        foregroundStyle(Palette.secondaryInk)
            .background(Palette.secondary.opacity(0.36), in: shape)
            .background(Color(.systemBackground), in: shape)
            .overlay { shape.strokeBorder(Palette.secondary.opacity(0.4)) }
    }
}
