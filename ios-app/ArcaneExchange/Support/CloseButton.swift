import SwiftUI

/// The mockup's `ISheet` close button: a plain "×", not a "Terminé"/"Fermer" label — used on
/// every Réglages sub-drawer's toolbar.
struct CloseButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: "xmark")
                .font(.subheadline.weight(.semibold))
        }
        .accessibilityLabel("Fermer")
    }
}

#Preview {
    CloseButton {}
        .padding()
}
