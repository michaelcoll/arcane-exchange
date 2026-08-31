import Charts
import SwiftUI

/// The 30-day Cardmarket price guide as a native Swift Charts figure: a cyan band from
/// `low` to `avg` with the `trend` line picked out on top — the SwiftUI counterpart of the
/// web `EnvelopeGraph`. Touch-and-drag scrubs a read-out, matching the web's hover tooltip.
struct PriceHistoryChart: View {
    let points: [PricePoint]

    @State private var scrubDate: Date?

    private var selected: PricePoint? {
        scrubDate.flatMap { PriceHistorySeries.nearest(to: $0, in: points) }
    }

    var body: some View {
        Chart {
            ForEach(points) { point in
                AreaMark(
                    x: .value("Date", point.date),
                    yStart: .value("Bas", point.low),
                    yEnd: .value("Moyenne", point.avg)
                )
                .interpolationMethod(.catmullRom)
                .foregroundStyle(
                    .linearGradient(
                        colors: [Color.accentColor.opacity(0.28), Color.accentColor.opacity(0.04)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )

                LineMark(
                    x: .value("Date", point.date),
                    y: .value("Tendance", point.trend)
                )
                .interpolationMethod(.catmullRom)
                .lineStyle(StrokeStyle(lineWidth: 2.4, lineCap: .round))
                .foregroundStyle(Color.accentColor)
            }

            if let selected {
                RuleMark(x: .value("Date", selected.date))
                    .foregroundStyle(.secondary.opacity(0.4))
                    .lineStyle(StrokeStyle(lineWidth: 1, dash: [3, 3]))
                    .annotation(
                        position: .top,
                        spacing: 6,
                        overflowResolution: .init(x: .fit(to: .chart), y: .disabled)
                    ) {
                        tooltip(for: selected)
                    }

                PointMark(
                    x: .value("Date", selected.date),
                    y: .value("Tendance", selected.trend)
                )
                .foregroundStyle(Color.accentColor)
                .symbolSize(60)
            }
        }
        .chartYScale(domain: PriceHistorySeries.yDomain(for: points))
        .chartYAxis {
            AxisMarks(position: .leading, values: .automatic(desiredCount: 4)) { value in
                AxisGridLine()
                AxisValueLabel {
                    if let euros = value.as(Double.self) {
                        Text(Price.euros(cents: Int(euros * 100)))
                    }
                }
            }
        }
        .chartXAxis {
            AxisMarks(values: .stride(by: .day, count: 7)) { _ in
                AxisGridLine()
                AxisValueLabel(format: .dateTime.day().month(.abbreviated))
            }
        }
        .chartXSelection(value: $scrubDate)
        .animation(.snappy, value: scrubDate)
    }

    private func tooltip(for point: PricePoint) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(point.date, format: .dateTime.day().month(.abbreviated))
                .font(.caption2)
                .foregroundStyle(.secondary)
            row("tendance", point.trend)
            row("moyenne", point.avg)
            row("bas", point.low)
        }
        .padding(8)
        // A `Chart` annotation floats with nothing behind it, so `.regularMaterial` has
        // nothing to blur and renders near-black. `BackgroundStyle` is an opaque,
        // theme-aware fill.
        .background(.background, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .strokeBorder(.quaternary)
        }
        .shadow(color: .black.opacity(0.18), radius: 6, y: 2)
    }

    private func row(_ label: String, _ value: Double) -> some View {
        HStack(spacing: 10) {
            Text(label)
                .foregroundStyle(.secondary)
            Spacer(minLength: 0)
            Text(Price.euros(cents: Int(value * 100)))
                .fontWeight(.semibold)
                .monospacedDigit()
                .foregroundStyle(.primary)
        }
        .font(.caption2)
    }
}
