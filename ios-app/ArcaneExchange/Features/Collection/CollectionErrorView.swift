import SwiftUI

/// Full-screen placeholder shown instead of the grid when the collection failed to load.
struct CollectionErrorView: View {
    let error: CollectionViewModel.LoadError
    let onRetry: () -> Void

    var body: some View {
        ContentUnavailableView(
            label: { Label("Collection indisponible", systemImage: "exclamationmark.triangle") },
            description: { Text(error.message) },
            actions: {
                Button("Réessayer", action: onRetry)
            }
        )
    }
}

#Preview {
    CollectionErrorView(error: .network, onRetry: {})
}
