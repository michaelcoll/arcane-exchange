import SwiftUI

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
