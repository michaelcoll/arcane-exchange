import NukeUI
import SwiftUI

/// One side of the trade, as the mockup's "table de jeu" rail (`tr-sh` + `tr-rail`): a header
/// naming who puts what down and what it is worth, then the cards laid side by side, face up.
///
/// The cards are shown full-frame and large rather than as list rows: what a player weighs in
/// a trade is the cards themselves, and a 116pt face is readable without opening anything.
struct TradeCardRail: View {
    /// The handle of whoever puts these cards down; `nil` is the user themselves.
    let owner: String?
    let cards: [TradeCard]
    /// Cards on both sides are locked into this trade — `tr-tile.locked`.
    let isReserved: Bool
    let emptyMessage: String
    /// `nil` on a side the user cannot compose: their own, or any side once the trade is
    /// locked. Non-nil puts a remove button on every tile.
    var onRemove: ((TradeCard) -> Void)?
    /// The trailing dashed slot that opens the partner's collection, when this side can grow.
    var addSlot: TradeRailAddSlot?

    private static let tileWidth: CGFloat = 116

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            header
            rail
        }
    }

    private var header: some View {
        HStack(spacing: 9) {
            if let owner {
                PlayerAvatar(username: owner, size: 26)
            }
            (ownerText + Text(" ") + Text("POSE").foregroundStyle(.secondary).tracking(0.6))
                .font(.system(size: 10.5, design: .monospaced))
                .foregroundStyle(.tertiary)
                .lineLimit(1)
                .truncationMode(.tail)

            Spacer(minLength: 8)

            Text(Price.euros(cents: TradeBalance.total(of: cards)))
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(Color.accentColor)
                .layoutPriority(1)
        }
        .padding(.horizontal, 4)
    }

    private var rail: some View {
        ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: 9) {
                ForEach(cards, id: \.lineID) { card in
                    TradeCardTile(
                        card: card,
                        width: Self.tileWidth,
                        isReserved: isReserved,
                        onRemove: onRemove
                    )
                }

                // The empty state is wordless once there's an add slot to explain the gap —
                // the slot itself is the invitation. Only a side that cannot grow (locked, or
                // the user's own — the partner composes it) falls back to the ghost tile.
                if cards.isEmpty, addSlot == nil {
                    TradeRailGhost(message: emptyMessage, width: Self.tileWidth)
                }

                if let addSlot {
                    NavigationLink(value: addSlot.route) {
                        TradeRailSlot(label: addSlot.label, width: Self.tileWidth)
                    }
                    .buttonStyle(.plain)
                }
            }
            // Room for the badges that overhang each tile's top-left corner.
            .padding(.top, 8)
            .padding(.leading, 8)
            .padding(.bottom, 3)
        }
        .scrollIndicators(.hidden)
        .scrollClipDisabled()
        .padding(.leading, -8)
    }

    private var ownerText: Text {
        Text(owner.map { "@\($0)".uppercased() } ?? "JE").tracking(1.4)
    }
}

/// Where the trailing "add a card" slot leads: the partner's tradable collection.
struct TradeRailAddSlot {
    let label: String
    let route: SearchResultsRoute
}

/// One card on the table: the face, its name, and what it is worth — plus the badge that says
/// what can be done with it, a remove button while the trade is open, a lock once it is
/// locked down.
private struct TradeCardTile: View {
    let card: TradeCard
    let width: CGFloat
    let isReserved: Bool
    let onRemove: ((TradeCard) -> Void)?

    private var value: Int {
        Int(card.price_guide?.trend ?? 0) * Int(card.quantity)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            face
            Text(card.name)
                .font(.system(size: 11.5))
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Text(Price.euros(cents: value))
                .font(.system(size: 12.5, weight: .semibold, design: .monospaced))
                .foregroundStyle(Color.accentColor)
        }
        .frame(width: width, alignment: .leading)
        .accessibilityElement(children: .combine)
    }

    private var face: some View {
        Color.clear
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .overlay { artwork }
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(.black.opacity(0.4)) }
            .overlay(alignment: .topTrailing) { quantityBadge }
            // The mockup dims a locked card rather than covering it: it is still the card you
            // are trading, just no longer yours to move.
            .opacity(isReserved && onRemove == nil ? 0.82 : 1)
            .shadow(color: .black.opacity(0.55), radius: 9, y: 6)
            .overlay(alignment: .topLeading) { cornerBadge }
            .frame(width: width)
    }

    private var artwork: some View {
        let url = CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)
        return LazyImage(url: url) { state in
            if let image = state.image {
                image.resizable().scaledToFill().foil(card.foil)
            } else {
                Rectangle().fill(.quaternary)
            }
        }
    }

    /// Overhangs the corner (`top:-7;left:-7`), ringed in the page colour so it reads as a
    /// control sitting on top of the card rather than printed on it.
    @ViewBuilder private var cornerBadge: some View {
        if let onRemove {
            Button {
                onRemove(card)
            } label: {
                badgeShape(fill: Color.red, foreground: .white) {
                    Image(systemName: "minus")
                }
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Retirer \(card.name)")
        } else if isReserved {
            badgeShape(fill: Color.purple.opacity(0.22), foreground: .purple) {
                Image(systemName: "lock.fill")
            }
            .accessibilityLabel("Carte réservée")
        }
    }

    private func badgeShape(
        fill: Color,
        foreground: Color,
        @ViewBuilder glyph: () -> some View
    ) -> some View {
        glyph()
            .font(.system(size: 11, weight: .bold))
            .foregroundStyle(foreground)
            .frame(width: 24, height: 24)
            .background(fill, in: .circle)
            .overlay { Circle().strokeBorder(Color(.systemBackground), lineWidth: 2) }
            .offset(x: -7, y: -7)
    }

    @ViewBuilder private var quantityBadge: some View {
        if card.quantity > 1 {
            Text("×\(card.quantity)")
                .font(.caption2.weight(.semibold))
                .monospacedDigit()
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(.ultraThinMaterial, in: .capsule)
                .padding(5)
        }
    }
}

/// The dashed border shared by the rail's two placeholder tiles (`tr-slot`, `tr-ghost`) — same
/// footprint as a card, same outline, different contents.
private struct TradeRailPlaceholder: ViewModifier {
    let width: CGFloat

    func body(content: Content) -> some View {
        content
            .frame(width: width, height: width * 7 / 5)
            .background {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(
                        Color.secondary.opacity(0.35),
                        style: StrokeStyle(lineWidth: 1.5, dash: [5, 4])
                    )
            }
    }
}

/// The dashed "add a card" tile that closes the rail (`tr-slot`).
private struct TradeRailSlot: View {
    let label: String
    let width: CGFloat

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 17))
            Text(label)
                .font(.system(size: 11))
                .multilineTextAlignment(.center)
        }
        .foregroundStyle(.secondary)
        .padding(.horizontal, 6)
        .modifier(TradeRailPlaceholder(width: width))
    }
}

/// Stands in for a side with nothing on it and no way to add anything (`tr-ghost`): the trade
/// is locked, or this is the user's own side, which the partner composes. Wordless by design —
/// `message` carries the explanation as an accessibility label instead of printed text, the
/// way the mockup keeps it to a title/aria-label on the icon.
private struct TradeRailGhost: View {
    let message: String
    let width: CGFloat

    var body: some View {
        Image(systemName: "rectangle.stack")
            .font(.system(size: 26))
            .foregroundStyle(.secondary)
            .opacity(0.55)
            .modifier(TradeRailPlaceholder(width: width))
            .accessibilityLabel(message)
    }
}

/// The hinge between the two rails (`tr-pivot`): a hairline on each side of a capsule that
/// doubles as the reservation indicator.
struct TradeRailPivot: View {
    let isReserved: Bool

    var body: some View {
        HStack(spacing: 10) {
            line
            Label(
                isReserved ? "échange réservé" : "échange",
                systemImage: isReserved ? "lock.fill" : "arrow.left.arrow.right"
            )
            .font(.system(size: 11.5, weight: .semibold, design: .monospaced))
            .lineLimit(1)
            .fixedSize()
            .foregroundStyle(isReserved ? Color.purple : .secondary)
            .padding(.horizontal, 13)
            .padding(.vertical, 6)
            .background(capsuleFill, in: .capsule)
            .overlay {
                Capsule().strokeBorder(isReserved ? Color.purple.opacity(0.4) : Color.secondary.opacity(0.25))
            }
            line
        }
        .padding(.horizontal, 2)
    }

    private var capsuleFill: AnyShapeStyle {
        isReserved ? AnyShapeStyle(Color.purple.opacity(0.14)) : AnyShapeStyle(.quaternary)
    }

    private var line: some View {
        Rectangle()
            .fill(Color.secondary.opacity(0.25))
            .frame(height: 1)
            .frame(maxWidth: .infinity)
    }
}

#Preview("Rails") {
    let card = { (name: String, trend: Int32, quantity: Int32) in
        TradeCard(
            collector_number: "243",
            foil: false,
            language_code: "fr",
            name: name,
            price_guide: .init(avg: nil, low: nil, trend: trend),
            quantity: quantity,
            scryfall_id: name,
            set_code: "FIN"
        )
    }

    return NavigationStack {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                TradeCardRail(
                    owner: "mizzix_42",
                    cards: [card("Sire of Seven Deaths", 3100, 1), card("The Soul Stone", 900, 2)],
                    isReserved: false,
                    emptyMessage: "Tu n'as demandé aucune carte pour l'instant.",
                    onRemove: { _ in },
                    addSlot: TradeRailAddSlot(
                        label: "Chercher chez lui",
                        route: SearchResultsRoute(target: .player(username: "mizzix_42"))
                    )
                )

                TradeRailPivot(isReserved: false)

                // Rien à demander pour l'instant : le créneau d'ajout suffit à l'expliquer,
                // pas de tuile fantôme à côté.
                TradeCardRail(
                    owner: "mizzix_42",
                    cards: [],
                    isReserved: false,
                    emptyMessage: "Tu n'as demandé aucune carte pour l'instant.",
                    onRemove: { _ in },
                    addSlot: TradeRailAddSlot(
                        label: "Chercher chez lui",
                        route: SearchResultsRoute(target: .player(username: "mizzix_42"))
                    )
                )

                TradeRailPivot(isReserved: true)

                TradeCardRail(
                    owner: "tanguy_a",
                    cards: [card("Black Market Connections", 1300, 1)],
                    isReserved: true,
                    emptyMessage: "@mizzix_42 n'a demandé aucune de tes cartes."
                )

                TradeRailPivot(isReserved: true)

                // Le côté de l'utilisateur, vide : personne d'autre à composer, donc pas de
                // créneau d'ajout — la tuile fantôme porte seule l'explication (en VoiceOver).
                TradeCardRail(
                    owner: nil,
                    cards: [],
                    isReserved: false,
                    emptyMessage: "@mizzix_42 n'a demandé aucune de tes cartes."
                )
            }
            .padding(16)
        }
    }
}
