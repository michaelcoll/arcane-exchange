import SwiftUI

/// Sub-drawer "Raretés" of the collection's filter drawer: none picked means no restriction.
struct CollectionRaritiesSheet: View {
    @Binding var selection: Set<RarityCode>

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(RarityCode.allCases, id: \.self) { rarity in
                        CheckRow(
                            title: rarity.label,
                            isSelected: selection.contains(rarity),
                            accessory: { RaritySymbol(rarity: rarity) },
                            action: { toggle(rarity) }
                        )
                    }
                }
            }
            .navigationTitle("Raretés")
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
        .presentationDetents([.medium, .large])
    }

    private func toggle(_ rarity: RarityCode) {
        if selection.contains(rarity) {
            selection.remove(rarity)
        } else {
            selection.insert(rarity)
        }
    }
}

#Preview {
    CollectionRaritiesSheet(selection: .constant([.R, .M]))
}
