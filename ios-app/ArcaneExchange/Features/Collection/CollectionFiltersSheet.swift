import SwiftUI

/// The mockup's "Filtrer" sheet: rarities, then the sets actually present in the collection.
///
/// Edits go straight to the bound filters, so the grid — and the count on the bottom button —
/// update while the sheet is still open.
struct CollectionFiltersSheet: View {
    @Binding var filters: CollectionFilters
    let sets: [SetInfo]
    let resultCount: Int

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section("Rareté") {
                    ForEach(RarityCode.allCases, id: \.self) { rarity in
                        row(title: rarity.label, code: nil, isSelected: filters.rarities.contains(rarity)) {
                            toggle(rarity)
                        }
                    }
                }

                if !sets.isEmpty {
                    Section("Set") {
                        ForEach(sets, id: \.code) { set in
                            row(title: set.name, code: set.code, isSelected: filters.sets.contains(set.code)) {
                                toggle(set.code)
                            }
                        }
                    }
                }
            }
            .navigationTitle("Filtrer")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Réinitialiser") { filters.clearAll() }
                        .disabled(filters.activeCount == 0)
                }
            }
            .safeAreaInset(edge: .bottom) {
                Button(action: { dismiss() }, label: {
                    Text("Afficher \(CollectionCopy.cardCount(resultCount))")
                        .fontWeight(.semibold)
                        .frame(maxWidth: .infinity)
                })
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .padding(.horizontal)
                .padding(.vertical, 10)
                .background(.bar)
            }
        }
    }

    private func row(title: String, code: String?, isSelected: Bool, toggle: @escaping () -> Void) -> some View {
        Button(action: toggle) {
            HStack(spacing: 12) {
                if let code {
                    Text(code)
                        .font(.caption)
                        .fontWeight(.semibold)
                        .monospaced()
                        .foregroundStyle(.tint)
                        .frame(minWidth: 42, alignment: .leading)
                }
                Text(title)
                    .foregroundStyle(.primary)
                Spacer(minLength: 8)
                if isSelected {
                    Image(systemName: "checkmark")
                        .fontWeight(.semibold)
                        .foregroundStyle(.tint)
                }
            }
        }
        // Without this the row reads as one big tinted link instead of a list row.
        .buttonStyle(.plain)
        .contentShape(.rect)
    }

    private func toggle(_ rarity: RarityCode) {
        if filters.rarities.contains(rarity) {
            filters.rarities.remove(rarity)
        } else {
            filters.rarities.insert(rarity)
        }
    }

    private func toggle(_ setCode: String) {
        if filters.sets.contains(setCode) {
            filters.sets.remove(setCode)
        } else {
            filters.sets.insert(setCode)
        }
    }
}

#Preview {
    CollectionFiltersSheet(
        filters: .constant(CollectionFilters(rarities: [.R])),
        sets: [
            SetInfo(code: "MH3", name: "Modern Horizons 3"),
            SetInfo(code: "LTR", name: "The Lord of the Rings")
        ],
        resultCount: 8
    )
}
