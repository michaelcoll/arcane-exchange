import SwiftUI
import WidgetKit

/// « Échanges en cours », medium size: the web home page's trades tile — how many trades are
/// active, and the first two. A row opens that trade, anywhere else the Échanges tab.
///
/// Lives with the app's sources (not only the extension's) so the unit tests can render it.
struct TradesWidgetView: View {
    let content: HomeWidgetContent<TradesSnapshot>

    var body: some View {
        Group {
            switch content {
            case let .snapshot(snapshot):
                TradesTile(snapshot: snapshot)
            case .signedOut:
                HomeWidgetMessage.signedOut
            case .unavailable:
                HomeWidgetMessage.unavailable
            }
        }
        .widgetURL(HomeWidgetLink.trades.url)
    }
}

private struct TradesTile: View {
    let snapshot: TradesSnapshot

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            header
            if snapshot.trades.isEmpty {
                Text("Aucun échange en cours.")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ForEach(snapshot.trades.prefix(TradesSnapshot.rowCount)) { trade in
                    Link(destination: HomeWidgetLink.trade(
                        TradeDetailRoute(id: trade.id, partnerUsername: trade.partnerUsername)
                    ).url) {
                        TradeWidgetRow(trade: trade)
                    }
                }
                Spacer(minLength: 0)
            }
        }
        .padding(14)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var header: some View {
        HStack(spacing: 6) {
            Text("Échanges en cours")
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .tracking(1.2)
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
            Text("\(snapshot.total)")
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.secondary)
            Spacer(minLength: 0)
            HStack(spacing: 2) {
                Text("voir tout")
                Image(systemName: "chevron.right")
                    .font(.system(size: 9, weight: .semibold))
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .lineLimit(1)
    }
}

private struct TradeWidgetRow: View {
    let trade: ActiveTrade

    var body: some View {
        HStack(spacing: 10) {
            Circle()
                .fill(Color(.tertiarySystemFill))
                .frame(width: 30, height: 30)
                .overlay {
                    Text(PlayerMonogram.initials(from: trade.partnerUsername))
                        .font(.system(size: 12, weight: .bold))
                        .foregroundStyle(.secondary)
                }

            VStack(alignment: .leading, spacing: 1) {
                UsernameLabel(username: trade.partnerUsername)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                Text(trade.cardCountsLabel)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 6)

            TradeStatusPill(status: TradeStatus(apiValue: trade.status))
                .widgetAccentable()
        }
        // A `Link` tints its label with the accent; the row keeps the label colors instead.
        // `Color.primary`, not the hierarchical `.primary`: that one resolves to the tint.
        .foregroundStyle(Color.primary)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(Color(.systemBackground), in: .rect(cornerRadius: 12))
        .overlay {
            RoundedRectangle(cornerRadius: 12).strokeBorder(Color(.separator))
        }
    }
}

extension TradesSnapshot {
    /// The widget gallery's sample, also used for the placeholder while the first timeline loads.
    static let sample = TradesSnapshot(total: 3, trades: [
        ActiveTrade(
            id: "t1", partnerUsername: "mizzix_42", myCardCount: 2, partnerCardCount: 1, status: "ONE_ACCEPTED"
        ),
        ActiveTrade(
            id: "t2", partnerUsername: "golgari.jo", myCardCount: 1, partnerCardCount: 3, status: "PENDING"
        )
    ])
}

#Preview("Échanges en cours") {
    TradesWidgetView(content: .snapshot(.sample))
        .frame(width: 338, height: 158)
        .background(HomeWidgetSurface.background, in: .rect(cornerRadius: 22))
}
