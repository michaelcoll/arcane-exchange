import SwiftUI

extension ShapeStyle where Self == Color {
    /// The design system's secondary accent (`--violet` / `--accent-2`, `#cdbdff`) — EDHREC,
    /// balances, reserved cards. SwiftUI's own `.purple` is a different hue entirely, so every
    /// violet in the app goes through this token.
    ///
    /// The asset carries the palette value in dark mode and the mockup's light-theme violet
    /// (`oklch(0.56 0.16 295)` → `#7d5cc7`) in light mode, where `#cdbdff` is too pale to read.
    static var violet: Color {
        Color("Violet")
    }

    /// `--violet-ink`: the readable violet for text and glyphs sitting *on* a violet-tinted
    /// fill. `.violet` itself is a fill and border colour — used as text on its own tint it
    /// falls short of 4.5:1, which is what made the reserved badge hard to read in light mode.
    static var violetInk: Color {
        Color("VioletInk")
    }
}

extension View {
    /// The mockup's `.tint-violet` box: a violet fill always carries a violet line.
    func tintViolet(in shape: some InsettableShape) -> some View {
        background(Color.violet.opacity(0.12), in: shape)
            .overlay { shape.strokeBorder(Color.violet.opacity(0.4)) }
    }

    /// The mockup's `.reserved-flag` chip (`styles.css`): an opaque violet-tinted surface so
    /// the badge never has to fight the artwork behind it, `--violet-ink` text, and a
    /// `--violet-line` border.
    func reservedBadgeChip(in shape: some InsettableShape) -> some View {
        // The violet layer sits on an opaque surface, not on the artwork: `.background`
        // stacks backwards, so the tint is applied first and the surface behind it.
        foregroundStyle(.violetInk)
            .background(Color.violet.opacity(0.36), in: shape)
            .background(Color(.systemBackground), in: shape)
            .overlay { shape.strokeBorder(Color.violet.opacity(0.4)) }
    }
}
