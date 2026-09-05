import SwiftUI

/// Échanges tab (`ScrTrades` in the iOS mockup): the trades the user is a party to, split
/// between the ones still live and the archive.
struct TradesView: View {
    @State private var model = TradesViewModel()
    @State private var path = NavigationPath()

    var body: some View {
        NavigationStack(path: $path) {
            content
                .navigationTitle("Échanges")
                .navigationBarTitleDisplayMode(.inline)
                .accountToolbar()
                .refreshable { await model.reload() }
                .task { await model.reload() }
                .tradeDestinations()
                .cardBrowsingDestinations()
                // The stack keeps the list alive while a trade is pushed, so `.task` never
                // re-runs on the way back: an acceptance or an abandon would leave a stale row.
                .onChange(of: path.isEmpty) { _, isRoot in
                    if isRoot {
                        Task { await model.reload() }
                    }
                }
        }
        .tradeNavigation(path: $path)
    }

    @ViewBuilder private var content: some View {
        if model.isLoading, model.trades.isEmpty {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = model.loadError, model.trades.isEmpty {
            ContentUnavailableView(
                label: { Label("Échanges indisponibles", systemImage: "exclamationmark.triangle") },
                description: { Text(error.message) },
                actions: { Button("Réessayer") { Task { await model.reload() } } }
            )
        } else if model.trades.isEmpty {
            ContentUnavailableView(
                "Aucun échange",
                systemImage: "arrow.left.arrow.right",
                description: Text("Cherche une carte, puis propose un échange à celui qui la possède.")
            )
        } else {
            list
        }
    }

    private var list: some View {
        List {
            Section {
                if model.visibleTrades.isEmpty {
                    Text(emptySegmentMessage)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .listRowSeparator(.hidden)
                }
                ForEach(model.visibleTrades, id: \.id) { trade in
                    NavigationLink(
                        value: TradeDetailRoute(id: trade.id, partnerUsername: trade.partner_username)
                    ) {
                        TradeSummaryRow(trade: trade)
                    }
                    .task { await model.loadMoreIfNeeded(displaying: trade) }
                }
            } header: {
                segmentPicker
                    .textCase(nil)
                    .listRowInsets(EdgeInsets(top: 4, leading: 0, bottom: 12, trailing: 0))
            } footer: {
                if model.segment == .ongoing {
                    Text("""
                    Un échange verrouillé réserve les cartes des deux côtés : elles sortent de \
                    tes autres échanges.
                    """)
                }
            }

            if model.isLoadingMore {
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .listRowSeparator(.hidden)
            }
        }
    }

    private var segmentPicker: some View {
        Picker("Filtre", selection: $model.segment) {
            Text("En cours (\(model.ongoing.count))").tag(TradesViewModel.Segment.ongoing)
            Text(TradesViewModel.Segment.past.label).tag(TradesViewModel.Segment.past)
        }
        .pickerStyle(.segmented)
    }

    private var emptySegmentMessage: String {
        model.segment == .ongoing
            ? "Aucun échange en cours."
            : "Aucun échange clôturé ou abandonné."
    }
}

/// One row of the list: who, how many cards each way, when, and where the trade stands.
private struct TradeSummaryRow: View {
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
                Text("\(trade.my_card_count) ⇄ \(trade.partner_card_count) · \(relativeDate)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer(minLength: 8)

            TradeStatusPill(status: status)
        }
        .padding(.vertical, 4)
    }

    private var relativeDate: String {
        TradesCopy.relativeDate(from: trade.updated_at)
    }
}

/// The status chip used by both the list and the detail header.
struct TradeStatusPill: View {
    let status: TradeStatus

    var body: some View {
        Label(status.label, systemImage: status.symbol)
            .font(.caption2.weight(.semibold))
            .labelStyle(.titleAndIcon)
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .background(status.tint.opacity(0.15), in: .capsule)
            .foregroundStyle(status.tint)
    }
}

#Preview {
    TradesView()
}
