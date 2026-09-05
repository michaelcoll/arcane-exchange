import SwiftUI

/// The mockup's `IGroup`: an uppercase caption over a rounded card of rows. Every block of
/// the card screen that is a list of facts is built from this.
struct CardDetailGroup<Content: View>: View {
    let header: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(header)
                .sectionCaptionStyle()
                .padding(.leading, 4)

            VStack(spacing: 0) { content }
                .padding(.horizontal, 14)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        }
    }
}

/// One row of a `CardDetailGroup`: a label on the left, a figure on the right.
struct CardDetailRow<Trailing: View>: View {
    let title: String
    @ViewBuilder let trailing: Trailing

    init(_ title: String, @ViewBuilder trailing: () -> Trailing) {
        self.title = title
        self.trailing = trailing()
    }

    var body: some View {
        HStack {
            Text(title)
            Spacer(minLength: 12)
            trailing
                .fontWeight(.medium)
                .monospacedDigit()
        }
        .font(.subheadline)
        .padding(.vertical, 12)
    }
}

#Preview {
    CardDetailGroup(header: "Dans ma collection") {
        CardDetailRow("Exemplaires") { Text("×2") }
        Divider()
        CardDetailRow("Prix d'achat") { Text("9,80 €") }
    }
    .padding()
}
