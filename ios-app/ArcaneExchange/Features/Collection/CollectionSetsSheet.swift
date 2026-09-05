import SwiftUI

/// Sub-drawer "Sets" of the collection's filter drawer: the sets actually present in the
/// collection, none picked meaning no restriction.
///
/// The search field narrows the list on the device — the whole set list came down with
/// `GET /collection/stats`, so there is nothing to ask the backend.
struct CollectionSetsSheet: View {
    @Binding var selection: Set<String>
    let sets: [SetInfo]

    @Environment(\.dismiss) private var dismiss
    @State private var query = ""

    private var visibleSets: [SetInfo] {
        SetSearch.filter(sets, matching: query)
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(visibleSets, id: \.code) { set in
                        CheckRow(
                            title: set.name,
                            isSelected: selection.contains(set.code),
                            accessory: { SetSymbol(setCode: set.code).foregroundStyle(.secondary) },
                            action: { toggle(set.code) }
                        )
                    }
                }
            }
            .overlay {
                if visibleSets.isEmpty, !query.isEmpty {
                    ContentUnavailableView.search(text: query)
                }
            }
            .searchable(
                text: $query,
                placement: .navigationBarDrawer(displayMode: .always),
                prompt: "Rechercher un set"
            )
            .navigationTitle("Sets")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    ResetButton { selection = [] }
                        .disabled(selection.isEmpty)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    CloseButton { dismiss() }
                }
            }
        }
        .presentationDetents([.large])
    }

    private func toggle(_ code: String) {
        if selection.contains(code) {
            selection.remove(code)
        } else {
            selection.insert(code)
        }
    }
}

#Preview {
    CollectionSetsSheet(
        selection: .constant(["MH3"]),
        sets: [
            SetInfo(code: "MH3", name: "Modern Horizons 3"),
            SetInfo(code: "LTR", name: "The Lord of the Rings"),
            SetInfo(code: "EOE", name: "Edge of Eternities"),
            SetInfo(code: "FDN", name: "Foundations"),
            SetInfo(code: "DFT", name: "Aetherdrift")
        ]
    )
}
