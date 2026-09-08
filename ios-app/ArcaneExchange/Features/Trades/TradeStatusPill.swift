import SwiftUI

/// The status chip used by both the list and the detail header.
struct TradeStatusPill: View {
    let status: TradeStatus

    var body: some View {
        Label(status.label, systemImage: status.symbol)
            .font(.caption2.weight(.semibold))
            .labelStyle(TradeStatusPillLabelStyle())
            .lineLimit(1)
            // A chip never wraps: in a list row the disclosure chevron eats the trailing
            // space, and without this the label breaks over two lines. The row's username
            // truncates instead — it is the elastic part, the status is not.
            .fixedSize(horizontal: true, vertical: false)
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .background(status.tint.opacity(0.15), in: .capsule)
            .foregroundStyle(status.tint)
    }
}

/// `.titleAndIcon` spaces the glyph and the label as a sentence would, which is far too airy
/// inside a chip this small — they read as one token here, not as icon + text.
private struct TradeStatusPillLabelStyle: LabelStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack(spacing: 3) {
            configuration.icon
            configuration.title
        }
    }
}

#Preview("Tous les statuts") {
    VStack(alignment: .leading, spacing: 10) {
        ForEach(TradeStatus.allCases, id: \.self) { status in
            TradeStatusPill(status: status)
        }
    }
    .padding(20)
}
