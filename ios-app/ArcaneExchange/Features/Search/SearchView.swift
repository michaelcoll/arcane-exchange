import SwiftUI

/// Rechercher tab — dedicated search (carte / decklist / joueur).
/// Scaffold only; content to be built against `GET /search/card` and friends.
struct SearchView: View {
    @Binding var searchText: String

    var body: some View {
        NavigationStack {
            ContentUnavailableView(
                "Rechercher une carte",
                systemImage: "magnifyingglass",
                description: Text("Par carte, decklist ou joueur.")
            )
            .navigationTitle("Rechercher")
        }
        .searchable(text: $searchText, prompt: "Vampiric Tutor…")
    }
}

#Preview {
    SearchView(searchText: .constant(""))
}
