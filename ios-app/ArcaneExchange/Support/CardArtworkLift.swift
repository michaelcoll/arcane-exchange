import SwiftUI

/// How card artwork is detached from the page, following the mockup's "mise en valeur des
/// cartes" block (`ios.css`): in dark mode the art sits on a tinted halo, in light mode —
/// where a halo would wash out against a pale background — it keeps its drop shadow.
///
/// The mockup tints the halo by colour identity; the API never returns one, so the tint comes
/// from the rarity colour the app already uses as a card's colour signal, and falls back to a
/// neutral light where even rarity is unknown (the mockup's own `--ink-3` default).
private struct CardArtworkLift: ViewModifier {
    @Environment(\.colorScheme) private var colorScheme

    let tint: Color
    let shadowOpacity: Double
    let shadowRadius: CGFloat
    let shadowY: CGFloat
    let haloRadius: CGFloat

    private var isDark: Bool {
        colorScheme == .dark
    }

    func body(content: Content) -> some View {
        content
            .shadow(color: .black.opacity(shadowOpacity), radius: shadowRadius, y: shadowY)
            .shadow(color: isDark ? tint.opacity(0.5) : .clear, radius: haloRadius)
    }
}

extension View {
    /// - Parameters:
    ///   - tint: halo colour in dark mode; pass `nil` when the card carries no rarity.
    ///   - shadowOpacity/shadowRadius/shadowY: the drop shadow, unchanged in both modes.
    ///   - haloRadius: spread of the dark-mode halo, scaled to the artwork size.
    func cardArtworkLift(
        tint: Color?,
        shadowOpacity: Double,
        shadowRadius: CGFloat,
        shadowY: CGFloat,
        haloRadius: CGFloat
    ) -> some View {
        modifier(
            CardArtworkLift(
                tint: tint ?? .white,
                shadowOpacity: shadowOpacity,
                shadowRadius: shadowRadius,
                shadowY: shadowY,
                haloRadius: haloRadius
            )
        )
    }
}
