import SwiftUI

/// One player's offer (`IRow` in the mockup), shared by the card screen's short owners list
/// and the full "Possesseurs" screen: avatar, handle, then quantity · price on the secondary
/// line, and either an "Échanger" pill or the lock of a copy already reserved by someone
/// else's accepted trade.
///
/// The whole row is the button, not just the pill — the mockup draws the pill as the affordance
/// but a thumb-sized target beats a 70-point one. The pill is therefore decoration, not a
/// nested control.
struct CardOfferRow: View {
    let offer: CardOffer
    let isStarting: Bool
    /// `nil` for a reserved copy, which cannot be asked for.
    let action: (() -> Void)?

    var body: some View {
        if let action {
            Button(action: action) { content }
                .buttonStyle(.plain)
                .disabled(isStarting)
        } else {
            content
        }
    }

    private var content: some View {
        HStack(spacing: 12) {
            PlayerAvatar(username: offer.owner_username, size: 34)
            VStack(alignment: .leading, spacing: 3) {
                Text("@\(offer.owner_username)")
                    .font(.subheadline.weight(.medium))
                    .lineLimit(1)
                subtitle
            }
            Spacer(minLength: 8)
            trailing
        }
        .padding(.vertical, 8)
        .contentShape(Rectangle())
    }

    private var subtitle: some View {
        HStack(spacing: 5) {
            Text("×\(offer.quantity)")
            if let price = offer.selling_price {
                separator
                Text(Price.euros(cents: Int(price)))
                    .fontWeight(.semibold)
                    .foregroundStyle(offer.reserved ? Color.secondary : Color.accentColor)
            }
            if offer.reserved {
                separator
                Text("réservée")
                    .foregroundStyle(.purple)
            }
        }
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .lineLimit(1)
    }

    private var separator: some View {
        Text("·").foregroundStyle(.tertiary)
    }

    @ViewBuilder private var trailing: some View {
        if isStarting {
            ProgressView()
        } else if action != nil {
            // Decoration: the row itself carries the tap, so this must not be a `Button`.
            Text("Échanger")
                .font(.footnote.weight(.semibold))
                .foregroundStyle(Color.accentColor)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(Color.accentColor.opacity(0.12), in: Capsule())
        } else if offer.reserved {
            Image(systemName: "lock.fill")
                .font(.footnote)
                .foregroundStyle(.purple)
        }
    }
}

#Preview("Disponible") {
    List {
        CardOfferRow(
            offer: .init(owner_username: "mizzix_42", quantity: 1, reserved: false, selling_price: 2950),
            isStarting: false,
            action: {}
        )
    }
}

#Preview("En cours") {
    List {
        CardOfferRow(
            offer: .init(owner_username: "selesnya_lea", quantity: 2, reserved: false, selling_price: 3100),
            isStarting: true,
            action: {}
        )
    }
}

#Preview("Réservée") {
    List {
        CardOfferRow(
            offer: .init(owner_username: "mono_black_max", quantity: 1, reserved: true, selling_price: 2880),
            isStarting: false,
            action: nil
        )
    }
}

#Preview("Sans prix") {
    List {
        CardOfferRow(
            offer: .init(owner_username: "guest", quantity: 1, reserved: false, selling_price: nil),
            isStarting: false,
            action: {}
        )
    }
}
