import SwiftUI

/// Clears a drawer's selection: an icon, paired with `CloseButton` on the opposite toolbar
/// edge. Call sites disable it when there is nothing to clear.
struct ResetButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: "arrow.counterclockwise")
                .font(.subheadline.weight(.semibold))
        }
        .accessibilityLabel("Réinitialiser")
    }
}

#Preview {
    ResetButton {}
        .padding()
}
