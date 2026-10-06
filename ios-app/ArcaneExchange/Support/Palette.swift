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
enum Palette {
    /// Also the system accent (`ASSETCATALOG_COMPILER_GLOBAL_ACCENT_COLOR_NAME`), so default
    /// tints, prominent buttons and toggles already use it.
    static var primary: Color {
        Color("Primary")
    }

    /// `--primary-ink`: text and glyphs sitting on a primary-tinted fill.
    static var primaryInk: Color {
        Color("PrimaryInk")
    }

    /// `--on-primary`: text and glyphs on a solid primary background.
    static var onPrimary: Color {
        Color("OnPrimary")
    }

    static var secondary: Color {
        Color("Secondary")
    }

    /// `--secondary-ink`: text and glyphs sitting on a secondary-tinted fill.
    static var secondaryInk: Color {
        Color("SecondaryInk")
    }

    /// `--on-secondary`: text and glyphs on a solid secondary background.
    static var onSecondary: Color {
        Color("OnSecondary")
    }

    /// `--down`: a value going down — a falling collection, a bad deal. Rising values are
    /// `primary`.
    static var down: Color {
        Color("Down")
    }
}

extension View {
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
