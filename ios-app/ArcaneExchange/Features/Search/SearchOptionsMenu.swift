import ArcaneExchangeCore
import SwiftUI

/// The "…" button of the Rechercher tab, like the Photos app: the axes, the sort and the
/// filters of the live card grid in one native menu. The sets, too many for a menu, open the
/// collection's searchable set sheet.
struct SearchOptionsMenu: View {
    @Bindable var model: SearchResultsViewModel

    @State private var isShowingSets = false

    var body: some View {
        Menu {
            Picker("Possession", selection: $model.axes.possession) {
                ForEach(PossessionAxis.allCases, id: \.self) { Text($0.label) }
            }
            .pickerStyle(.inline)

            if model.target.offersOfferAxis {
                Picker("Offre", selection: $model.axes.offer) {
                    ForEach(OfferAxis.allCases, id: \.self) { Text($0.label) }
                }
                .pickerStyle(.inline)
            }

            Section {
                sortMenu
                raritiesMenu
                Button("Sets…", systemImage: "square.stack.3d.up") { isShowingSets = true }
                    .disabled(model.sets.isEmpty)
            }

            if model.filters.activeCount > 0 {
                Section {
                    // Clears the filters only: the sort is kept.
                    Button("Effacer les filtres", systemImage: "xmark.circle", role: .destructive) {
                        model.filters.clearAll()
                    }
                }
            }
        } label: {
            Label("Options de recherche", systemImage: "ellipsis")
        }
        .sheet(isPresented: $isShowingSets) {
            CollectionSetsSheet(selection: $model.filters.sets, sets: model.sets)
        }
    }

    private var sortMenu: some View {
        Menu {
            let options = model.target.sortOptions
            if options.count > 1 {
                Picker("Trier par", selection: $model.filters.sortBy) {
                    ForEach(options, id: \.self) { Text($0.label).tag($0) }
                }
                .pickerStyle(.inline)
            }
            Picker("Ordre", selection: $model.filters.sortDir) {
                ForEach([SortDirection.desc, .asc], id: \.self) { direction in
                    Label(direction.label, systemImage: direction.icon).tag(direction)
                }
            }
            .pickerStyle(.inline)
        } label: {
            Label("Trier par", systemImage: model.filters.sortDir.icon)
            Text(model.filters.sortBy.label)
        }
    }

    /// Stays open between two ticks: several rarities are usually picked in a row.
    private var raritiesMenu: some View {
        Menu {
            ForEach(RarityCode.allCases, id: \.self) { rarity in
                Toggle(rarity.label, isOn: rarityBinding(rarity))
            }
        } label: {
            Label("Raretés", systemImage: "line.3.horizontal.decrease")
            Text(CollectionCopy.facetSelection(
                selected: model.filters.rarities.count,
                total: RarityCode.allCases.count,
                noneSelected: "Toutes"
            ))
        }
        .menuActionDismissBehavior(.disabled)
    }

    private func rarityBinding(_ rarity: RarityCode) -> Binding<Bool> {
        Binding(
            get: { model.filters.rarities.contains(rarity) },
            set: { isOn in
                if isOn {
                    model.filters.rarities.insert(rarity)
                } else {
                    model.filters.rarities.remove(rarity)
                }
            }
        )
    }
}
