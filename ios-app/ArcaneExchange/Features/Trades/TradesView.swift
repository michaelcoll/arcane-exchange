import SwiftUI

/// Échanges tab (`ScrTrades` in the iOS mockup): the trades the user is a party to, split
/// between the ones still live and the archive.
///
/// No `#Preview` here on purpose: the screen builds a Clerk-backed `accountToolbar()` and hits
/// the API on `.task`, which hangs and crashes the Xcode preview agent. The pieces it composes
/// (`TradesList`, `TradeSummaryRow`, `TradeStatusPill`) carry the previews instead.
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
            TradesList(
                segment: $model.segment,
                ongoingCount: model.ongoing.count,
                trades: model.visibleTrades,
                isLoadingMore: model.isLoadingMore,
                onRowAppear: { await model.loadMoreIfNeeded(displaying: $0) }
            )
        }
    }
}
