import SwiftUI

/// Trend headlined on the left, low and average as a small aside on the right — the mockup's
/// `pxhead`, which gives the number people actually read the most room — over the 30-day chart.
struct CardPriceGuide: View {
    let guide: PriceGuide?
    let history: CardDetailViewModel.HistoryState

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .center, spacing: 12) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("tendance")
                        .font(.caption2.monospaced())
                        .textCase(.uppercase)
                        .foregroundStyle(.secondary)
                    Text(price(guide?.trend))
                        .font(.system(size: 30, weight: .semibold).monospacedDigit())
                }
                Spacer(minLength: 0)
                VStack(alignment: .trailing, spacing: 3) {
                    aside("bas", cents: guide?.low)
                    aside("moyenne", cents: guide?.avg)
                }
            }
            chartArea
                .frame(height: 150)
                .animation(.snappy, value: history)
            Text("30 derniers jours · Cardmarket")
                .font(.caption2.monospaced())
                .foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func aside(_ label: String, cents: Int32?) -> some View {
        HStack(spacing: 5) {
            Text(label)
                .foregroundStyle(.secondary)
            Text(price(cents))
                .fontWeight(.semibold)
        }
        .font(.caption.monospaced())
    }

    private func price(_ cents: Int32?) -> String {
        cents.map { Price.euros(cents: Int($0)) } ?? "—"
    }

    @ViewBuilder private var chartArea: some View {
        switch history {
        case .loading:
            placeholder { ProgressView() }
        case let .ready(points):
            PriceHistoryChart(points: points)
                .transition(.opacity)
        case .notEnoughData:
            placeholder { Text("Pas encore assez d'historique") }
        case .failed:
            placeholder { Text("Historique indisponible") }
        }
    }

    private func placeholder(@ViewBuilder _ content: () -> some View) -> some View {
        content()
            .font(.caption2.monospaced())
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
