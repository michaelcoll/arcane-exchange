import SwiftUI

/// The Échanges feed itself: the segment picker as the section header, one row per trade, and
/// the pagination spinner at the bottom.
///
/// Data-driven rather than model-driven so it renders in a preview — the screen around it
/// needs a signed-in Clerk session, this does not.
struct TradesList: View {
    @Binding var segment: TradesViewModel.Segment
    let ongoingCount: Int
    /// The trades of the selected segment.
    let trades: [TradeSummary]
    let isLoadingMore: Bool
    /// Called as each row appears, to fetch the next page near the end of the feed.
    let onRowAppear: (TradeSummary) async -> Void

    var body: some View {
        List {
            Section {
                if trades.isEmpty {
                    Text(emptySegmentMessage)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .listRowSeparator(.hidden)
                }
                ForEach(trades, id: \.id) { trade in
                    NavigationLink(
                        value: TradeDetailRoute(id: trade.id, partnerUsername: trade.partner_username)
                    ) {
                        TradeSummaryRow(trade: trade)
                    }
                    .task { await onRowAppear(trade) }
                }
            } header: {
                segmentPicker
                    .textCase(nil)
                    .listRowInsets(EdgeInsets(top: 4, leading: 0, bottom: 12, trailing: 0))
            } footer: {
                if segment == .ongoing {
                    Text("""
                    Un échange verrouillé réserve les cartes des deux côtés : elles sortent de \
                    tes autres échanges.
                    """)
                }
            }

            if isLoadingMore {
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .listRowSeparator(.hidden)
            }
        }
    }

    private var segmentPicker: some View {
        Picker("Filtre", selection: $segment) {
            Text("En cours (\(ongoingCount))").tag(TradesViewModel.Segment.ongoing)
            Text(TradesViewModel.Segment.past.label).tag(TradesViewModel.Segment.past)
        }
        .pickerStyle(.segmented)
    }

    private var emptySegmentMessage: String {
        segment == .ongoing
            ? "Aucun échange en cours."
            : "Aucun échange clôturé ou abandonné."
    }
}

#Preview("Feed") {
    @Previewable @State var segment = TradesViewModel.Segment.ongoing

    NavigationStack {
        TradesList(
            segment: $segment,
            ongoingCount: TradeSummary.previewFeed.count { TradeStatus(apiValue: $0.status).isOngoing },
            trades: TradeSummary.previewFeed.filter {
                TradeStatus(apiValue: $0.status).isOngoing == (segment == .ongoing)
            },
            isLoadingMore: false,
            onRowAppear: { _ in }
        )
        .navigationTitle("Échanges")
        .navigationBarTitleDisplayMode(.inline)
    }
}

#Preview("Segment vide") {
    @Previewable @State var segment = TradesViewModel.Segment.past

    NavigationStack {
        TradesList(
            segment: $segment,
            ongoingCount: 3,
            trades: [],
            isLoadingMore: true,
            onRowAppear: { _ in }
        )
        .navigationTitle("Échanges")
        .navigationBarTitleDisplayMode(.inline)
    }
}
