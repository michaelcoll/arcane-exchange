import ArcaneExchangeCore
import SwiftUI

/// Zero to five stars, tapped once. Only shown while the user has not rated yet, so it has no
/// selected state to carry.
struct TradeRatingStars: View {
    let onRate: (Int) -> Void

    var body: some View {
        HStack(spacing: 6) {
            ForEach(1 ... 5, id: \.self) { value in
                Button {
                    onRate(value)
                } label: {
                    Image(systemName: "star")
                        .font(.title3)
                        .foregroundStyle(Palette.secondary)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("\(value) étoile\(value > 1 ? "s" : "")")
            }
        }
    }
}

/// The bottom action of the trade screen: one decision per status, as the mockup's `actbar`
/// does — never a wall of buttons, and only ever this one. Abandoning lives in the nav bar's
/// "…" menu instead of a second button here: this screen hides the tab bar precisely so this
/// area can read as a single, focused commitment rather than a toolbar of options. Nothing at
/// all once the trade is closed or abandoned.
struct TradeActionBar: View {
    let action: TradePrimaryAction
    let partnerUsername: String
    let acceptLabel: String
    let isBusy: Bool
    /// The trade has loaded — until then there is nothing to act on.
    let isReady: Bool
    let onAccept: () -> Void
    let onConfirm: () -> Void

    var body: some View {
        if isReady, action != .none {
            primaryAction
                .disabled(isBusy)
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .background(.bar)
        }
    }

    @ViewBuilder private var primaryAction: some View {
        switch action {
        case .accept:
            Button(acceptLabel, systemImage: "checkmark", action: onAccept)
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .foregroundStyle(Palette.onPrimary)
                .frame(maxWidth: .infinity)

        case .awaitingAcceptance:
            waitingLabel("En attente de ", systemImage: "clock", tint: .secondary)

        case .confirmExchange:
            Button("Confirmer « échange réalisé »", systemImage: "checkmark", action: onConfirm)
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .tint(Palette.secondary)
                .foregroundStyle(Palette.onSecondary)
                .frame(maxWidth: .infinity)

        case .awaitingConfirmation:
            waitingLabel(
                "Confirmé, en attente de ",
                systemImage: "checkmark",
                tint: Palette.secondary
            )

        case .none:
            EmptyView()
        }
    }

    /// `lead` followed by the partner's handle, which always carries the secondary role.
    private func waitingLabel(_ lead: String, systemImage: String, tint: Color) -> some View {
        Label {
            Text(lead) + Text("@\(partnerUsername)").foregroundStyle(Palette.secondary)
        } icon: {
            Image(systemName: systemImage)
        }
        .font(.subheadline.weight(.medium))
        .foregroundStyle(tint)
        .frame(maxWidth: .infinity)
    }
}

#Preview("Étoiles") {
    TradeRatingStars { _ in }
        .padding()
}

#Preview("Barre d'action") {
    VStack(spacing: 20) {
        TradeActionBar(
            action: .accept,
            partnerUsername: "mizzix_42",
            acceptLabel: "Accepter et payer 21 €",
            isBusy: false,
            isReady: true,
            onAccept: {},
            onConfirm: {}
        )
        TradeActionBar(
            action: .awaitingAcceptance,
            partnerUsername: "mizzix_42",
            acceptLabel: "Accepter",
            isBusy: false,
            isReady: true,
            onAccept: {},
            onConfirm: {}
        )
        TradeActionBar(
            action: .confirmExchange,
            partnerUsername: "mizzix_42",
            acceptLabel: "Accepter",
            isBusy: false,
            isReady: true,
            onAccept: {},
            onConfirm: {}
        )
        TradeActionBar(
            action: .awaitingConfirmation,
            partnerUsername: "mizzix_42",
            acceptLabel: "Accepter",
            isBusy: false,
            isReady: true,
            onAccept: {},
            onConfirm: {}
        )
    }
    .frame(maxHeight: .infinity, alignment: .bottom)
}
