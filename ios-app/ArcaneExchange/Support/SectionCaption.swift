import SwiftUI

/// The small-caps section title used throughout the app for the mockup's `IGroup` header: an
/// uppercase caption, secondary-colored. Originates in `CardDetailView`'s grouped lists.
struct SectionCaptionStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(.caption)
            .textCase(.uppercase)
            .foregroundStyle(.secondary)
    }
}

extension View {
    func sectionCaptionStyle() -> some View {
        modifier(SectionCaptionStyle())
    }
}

#Preview {
    Text("Ce que je propose à l'échange")
        .sectionCaptionStyle()
        .padding()
}
