import SwiftUI

/// Échanges tab — list of trades (en cours / historique).
/// Scaffold only; content to be built against `GET /trades`.
struct TradesView: View {
    var body: some View {
        NavigationStack {
            ContentUnavailableView(
                "Échanges à venir",
                systemImage: "arrow.left.arrow.right",
                description: Text("Les échanges en cours et l'historique s'afficheront ici.")
            )
            .navigationTitle("Échanges")
        }
    }
}

#Preview {
    TradesView()
}
