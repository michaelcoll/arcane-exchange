import SwiftUI

/// The mockup's "Filtrer" drawer, as a summary: one row per facet, each opening its own
/// sub-drawer — the same shape as the Réglages screen.
///
/// Edits go straight to the bound filters, so the grid behind the drawer reloads while it is
/// still open.
struct CollectionFiltersSheet: View {
    /// The sub-drawers, one per facet.
    private enum Drawer: String, Identifiable {
        case sets
        case rarities

        var id: String {
            rawValue
        }
    }

    @Binding var filters: CollectionFilters
    let sets: [SetInfo]

    @Environment(\.dismiss) private var dismiss
    @State private var drawer: Drawer?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    DrawerRow(
                        title: "Sets",
                        systemImage: "square.stack.3d.up",
                        value: CollectionCopy.facetSelection(
                            selected: filters.sets.count,
                            total: sets.count,
                            noneSelected: "Tous"
                        )
                    ) {
                        drawer = .sets
                    }
                    .disabled(sets.isEmpty)

                    DrawerRow(
                        title: "Raretés",
                        systemImage: "line.3.horizontal.decrease",
                        value: CollectionCopy.facetSelection(
                            selected: filters.rarities.count,
                            total: RarityCode.allCases.count,
                            noneSelected: "Toutes"
                        )
                    ) {
                        drawer = .rarities
                    }
                }
            }
            .navigationTitle("Filtrer")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    ResetButton { filters.clearAll() }
                        .disabled(filters.activeCount == 0)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    CloseButton { dismiss() }
                }
            }
            .sheet(item: $drawer) { drawer in
                switch drawer {
                case .sets: CollectionSetsSheet(selection: $filters.sets, sets: sets)
                case .rarities: CollectionRaritiesSheet(selection: $filters.rarities)
                }
            }
        }
        .presentationDetents([.medium, .large])
    }
}

#Preview {
    CollectionFiltersSheet(
        filters: .constant(CollectionFilters(rarities: [.R])),
        sets: [
            SetInfo(code: "MH3", name: "Modern Horizons 3"),
            SetInfo(code: "LTR", name: "The Lord of the Rings")
        ]
    )
}
