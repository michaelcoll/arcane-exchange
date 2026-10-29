import SwiftUI

/// Full-screen placeholder shown instead of the grid when the collection failed to load.
struct CollectionErrorView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        ContentUnavailableView(
            label: { Label("Collection indisponible", systemImage: "exclamationmark.triangle") },
            description: { Text(message) },
            actions: {
                Button("Réessayer", action: onRetry)
            }
        )
    }
}

#Preview {
    CollectionErrorView(message: APIErrorMessages.network, onRetry: {})
}
