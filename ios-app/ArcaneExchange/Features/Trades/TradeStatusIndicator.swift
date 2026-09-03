import SwiftUI

/// Where the trade stands, as the mockup's `tr-dots` row: the five lifecycle steps as dots,
/// with the one in progress swollen into a labelled capsule in its own position.
///
/// The shape carries the meaning before the words do — how many dots sit to the left of the
/// capsule is how far along the trade is — so it stays one line, centred, above the cards.
struct TradeStatusIndicator: View {
    let status: TradeStatus
    /// When set, a trailing ⓘ pushes the steps explainer. Left `nil` where there is no stack
    /// to push onto (previews, or a read-only use).
    var stepsRoute: TradeStepsRoute?

    private static let dotSize: CGFloat = 6
    private static let spacing: CGFloat = 7

    var body: some View {
        HStack(spacing: Self.spacing) {
            if let index = status.lifecycleIndex {
                dots(count: index, isDone: true)
                capsuleLabel
                dots(count: TradeStatus.lifecycle.count - index - 1, isDone: false)
            } else {
                // Abandoned left the nominal path: no step is lit, and the capsule trails the
                // dots with no "n/5" to quote.
                dots(count: TradeStatus.lifecycle.count, isDone: false)
                capsuleLabel
            }

            if let stepsRoute {
                NavigationLink(value: stepsRoute) { helpButton }
                    .buttonStyle(.plain)
            }
        }
        // Ideal size first, centred second. Without `fixedSize` the row's spare width is
        // handed to the only flexible child — the capsule's label — which stretches its
        // background a hair past the stroked border and leaves a visible stub.
        .fixedSize(horizontal: true, vertical: false)
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAddTraits(stepsRoute == nil ? [] : .isButton)
    }

    /// Steps already behind the trade stay the accent colour whatever the current tone is —
    /// the path travelled does not change colour when the trade turns violet or green.
    private func dots(count: Int, isDone: Bool) -> some View {
        ForEach(0 ..< max(count, 0), id: \.self) { _ in
            Circle()
                .fill(isDone ? Color.accentColor.opacity(0.38) : Color.secondary.opacity(0.3))
                .frame(width: Self.dotSize, height: Self.dotSize)
        }
    }

    private var capsuleLabel: some View {
        Text(label)
            .font(.system(size: 10, weight: .medium, design: .monospaced))
            .tracking(1)
            .foregroundStyle(status.tint)
            .lineLimit(1)
            .padding(.horizontal, 11)
            .padding(.vertical, 4)
            .background(status.tint.opacity(0.14), in: .capsule)
            .overlay { Capsule().strokeBorder(status.tint.opacity(0.4), lineWidth: 1) }
    }

    private var helpButton: some View {
        Image(systemName: "info")
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(.secondary)
            .frame(width: 24, height: 24)
            .overlay { Circle().strokeBorder(Color.secondary.opacity(0.3), lineWidth: 1) }
    }

    /// "NÉGOCIATION · 1/5" — the step number only makes sense on the nominal path.
    private var label: String {
        guard let index = status.lifecycleIndex else { return status.label.uppercased() }
        return "\(status.label) · \(index + 1)/\(TradeStatus.lifecycle.count)".uppercased()
    }

    private var accessibilityLabel: String {
        guard let index = status.lifecycleIndex else { return status.label }
        return "\(status.label), étape \(index + 1) sur \(TradeStatus.lifecycle.count)"
    }
}

#Preview("Tous les statuts") {
    NavigationStack {
        VStack(spacing: 22) {
            ForEach(TradeStatus.allCases, id: \.self) { status in
                TradeStatusIndicator(
                    status: status,
                    stepsRoute: TradeStepsRoute(status: status, partnerUsername: "mizzix_42")
                )
            }

            Divider().padding(.vertical, 6)

            // Without the ⓘ — the shape has to hold up on its own.
            TradeStatusIndicator(status: .oneAccepted)
        }
        .padding(20)
        .navigationTitle("@mizzix_42")
        .navigationBarTitleDisplayMode(.inline)
        .navigationDestination(for: TradeStepsRoute.self) { TradeStepsView(route: $0) }
    }
}
