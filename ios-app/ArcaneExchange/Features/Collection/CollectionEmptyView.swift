import SwiftUI

/// Full-screen placeholder shown instead of the grid when there is nothing to show — the copy
/// differs depending on whether filters narrowed an otherwise non-empty collection to nothing,
/// or the collection itself is empty.
struct CollectionEmptyView: View {
    let hasActiveFilters: Bool
    let onClearFilters: () -> Void

    var body: some View {
        if hasActiveFilters {
            ContentUnavailableView(
                label: { Label("Aucune carte", systemImage: "line.3.horizontal.decrease") },
                description: { Text("Aucune carte de ta collection ne correspond à ces filtres.") },
                actions: {
                    Button("Réinitialiser les filtres", action: onClearFilters)
                }
            )
        } else {
            ContentUnavailableView(
                "Collection vide",
                systemImage: "rectangle.stack",
                description: Text("Importe ton fichier ManaBox pour voir tes cartes ici.")
            )
        }
    }
}

#Preview("Filtres actifs") {
    CollectionEmptyView(hasActiveFilters: true, onClearFilters: {})
}

#Preview("Collection vide") {
    CollectionEmptyView(hasActiveFilters: false, onClearFilters: {})
}
