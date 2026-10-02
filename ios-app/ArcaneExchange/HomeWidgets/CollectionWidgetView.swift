import Charts
import SwiftUI
import WidgetKit

/// « Ma collection », medium size: the web home page's collection tile — the 30-day value
/// graph full-bleed behind a fading scrim, the value and its variation on top, the card counts
/// at the bottom. The whole widget opens the Collection tab.
///
/// Lives with the app's sources (not only the extension's) so the unit tests can render it.
struct CollectionWidgetView: View {
    let content: HomeWidgetContent<CollectionSnapshot>

    var body: some View {
        Group {
            switch content {
            case let .snapshot(snapshot):
                CollectionTile(snapshot: snapshot)
            case .signedOut:
                HomeWidgetMessage.signedOut
            case .unavailable:
                HomeWidgetMessage.unavailable
            }
        }
        .widgetURL(HomeWidgetLink.collection.url)
    }
}

private struct CollectionTile: View {
    let snapshot: CollectionSnapshot

    var body: some View {
        ZStack {
            graph
            scrims
            VStack(alignment: .leading, spacing: 0) {
                kpis
                Spacer(minLength: 0)
                counts
            }
            .padding(16)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder private var graph: some View {
        if snapshot.hasEnoughHistory {
            CollectionValueGraph(points: snapshot.points)
                // Room under the KPIs: the curve lives in the lower part of the tile.
                .padding(.top, 36)
        }
    }

    /// The web's two scrims: the graph fades out under the KPIs, and darkens again behind the
    /// counts.
    private var scrims: some View {
        VStack(spacing: 0) {
            LinearGradient(
                stops: [
                    .init(color: HomeWidgetSurface.background.opacity(0.95), location: 0),
                    .init(color: HomeWidgetSurface.background.opacity(0.55), location: 0.5),
                    .init(color: HomeWidgetSurface.background.opacity(0), location: 1)
                ],
                startPoint: .top,
                endPoint: .bottom
            )
            Spacer(minLength: 0)
            LinearGradient(
                colors: [HomeWidgetSurface.background.opacity(0), HomeWidgetSurface.background.opacity(0.95)],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: 44)
        }
        .allowsHitTesting(false)
    }

    private var kpis: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text("Ma collection · CardMarket")
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .tracking(1.2)
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Text(Price.euros(cents: snapshot.valueCents))
                .font(.system(size: 28, weight: .semibold))
                .monospacedDigit()
                .minimumScaleFactor(0.6)
                .lineLimit(1)
                .widgetAccentable()
            variation
                .font(.system(size: 12, design: .monospaced))
                .lineLimit(1)
                .minimumScaleFactor(0.8)
        }
    }

    /// Without a graph the variation is always « +0,0 % »: the web tile's placeholder takes
    /// its line instead — a medium widget has no room to center it behind the KPIs.
    @ViewBuilder private var variation: some View {
        if snapshot.hasEnoughHistory {
            Text(snapshot.variation.label)
                .foregroundStyle(snapshot.variation.isRising ? Palette.primary : Palette.down)
        } else {
            Text("Pas encore assez d'historique")
                .textCase(.uppercase)
                .foregroundStyle(.tertiary)
        }
    }

    private var counts: some View {
        HStack(spacing: 8) {
            count(snapshot.totalCards, "cartes")
            Circle()
                .fill(.tertiary)
                .frame(width: 3, height: 3)
            count(snapshot.uniqueCards, "uniques")
        }
        .font(.system(size: 11, design: .monospaced))
        .foregroundStyle(.secondary)
        .lineLimit(1)
    }

    /// « **1 234** cartes »: the number stands out, as on the web.
    private func count(_ value: Int, _ label: String) -> some View {
        let number = Text(value.formatted(.number.locale(Locale(identifier: "fr_FR"))))
            .fontWeight(.semibold)
            .foregroundStyle(.primary)
        return Text("\(number) \(label)")
    }
}

/// The value history drawn as the web's `EnvelopeGraph`, stripped of axes: a primary band from
/// `low` to `avg` and the `trend` line on top.
private struct CollectionValueGraph: View {
    let points: [PricePoint]

    var body: some View {
        Chart(points) { point in
            AreaMark(
                x: .value("Date", point.date),
                yStart: .value("Bas", point.low),
                yEnd: .value("Moyenne", point.avg)
            )
            .interpolationMethod(.catmullRom)
            .foregroundStyle(
                .linearGradient(
                    colors: [Palette.primary.opacity(0.28), Palette.primary.opacity(0.04)],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )

            LineMark(
                x: .value("Date", point.date),
                y: .value("Tendance", point.trend)
            )
            .interpolationMethod(.catmullRom)
            .lineStyle(StrokeStyle(lineWidth: 2, lineCap: .round))
            .foregroundStyle(Palette.primary)
        }
        .chartYScale(domain: PriceHistorySeries.yDomain(for: points))
        .chartXAxis(.hidden)
        .chartYAxis(.hidden)
        .chartLegend(.hidden)
        .widgetAccentable()
    }
}

/// The surface both widgets sit on, and that the collection scrims fade into.
enum HomeWidgetSurface {
    static var background: Color {
        Color(.secondarySystemBackground)
    }
}

/// The two states a widget can be in without data.
@MainActor
enum HomeWidgetMessage {
    static var signedOut: some View {
        message("Connecte-toi dans Arcane Exchange", systemImage: "person.crop.circle")
    }

    static var unavailable: some View {
        message("Serveur injoignable pour le moment", systemImage: "wifi.exclamationmark")
    }

    private static func message(_ text: String, systemImage: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: systemImage)
                .font(.title2)
                .foregroundStyle(Palette.primary)
                .widgetAccentable()
            Text(text)
                .font(.footnote.weight(.medium))
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

extension CollectionSnapshot {
    /// The widget gallery's sample, also used for the placeholder while the first timeline loads.
    static let sample: CollectionSnapshot = {
        let trends: [Int64] = [118_000, 119_400, 118_900, 120_300, 121_000, 120_400, 122_800, 123_456]
        let history = trends.enumerated().map { day, trend in
            PriceHistoryEntry(
                avg: trend + 2400,
                date: String(format: "2026-09-%02d", day * 4 + 1),
                low: trend - 3100,
                trend: trend
            )
        }
        return CollectionSnapshot(history: history, totalCards: 1234, uniqueCards: 456)
    }()
}

#Preview("Ma collection") {
    CollectionWidgetView(content: .snapshot(.sample))
        .frame(width: 338, height: 158)
        .background(HomeWidgetSurface.background, in: .rect(cornerRadius: 22))
}
