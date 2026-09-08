import SwiftUI

/// One row of the Échanges list: who, how many cards each way, when, and where the trade stands.
struct TradeSummaryRow: View {
    let trade: TradeSummary

    private var status: TradeStatus {
        TradeStatus(apiValue: trade.status)
    }

    var body: some View {
        HStack(spacing: 12) {
            PlayerAvatar(username: trade.partner_username)

            VStack(alignment: .leading, spacing: 3) {
                UsernameLabel(username: trade.partner_username)
                    .fontWeight(.semibold)
                    .lineLimit(1)
                Text("\(trade.my_card_count) ⇄ \(trade.partner_card_count)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(relativeDate)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            Spacer(minLength: 8)

            TradeStatusPill(status: status)
        }
        .padding(.vertical, 4)
    }

    /// When the trade last moved — its own line, under the card counts.
    private var relativeDate: String {
        TradesCopy.relativeDate(from: trade.updated_at)
    }
}

#Preview("Lignes") {
    NavigationStack {
        List {
            // Pushed rows, so the disclosure chevron is in the layout: it is what squeezes
            // the status pill.
            ForEach(TradeSummary.previewFeed, id: \.id) { trade in
                NavigationLink(value: trade.id) {
                    TradeSummaryRow(trade: trade)
                }
            }
        }
        .navigationTitle("Échanges")
        .navigationBarTitleDisplayMode(.inline)
    }
}
