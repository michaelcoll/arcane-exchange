import SwiftUI

/// Sort menu + filter chip, scrolling with the grid like the mockup's chip rail — the filters
/// are a refinement of the list, not app chrome.
struct CollectionFilterRail: View {
    @Binding var filters: CollectionFilters
    /// The criteria the sort menu offers. With a single one, the menu only picks the direction.
    let sortOptions: [SortField]
    let onFilterTap: () -> Void

    var body: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 8) {
                sortMenu
                chipButton(
                    title: CollectionCopy.filterChip(activeCount: filters.activeCount),
                    systemImage: "line.3.horizontal.decrease",
                    isActive: filters.activeCount > 0,
                    action: onFilterTap
                )
            }
            .padding(.vertical, 2)
        }
        .scrollIndicators(.hidden)
        .scrollClipDisabled()
    }

    /// The arrow, not the wording, carries the direction — so the chip's icon flips with it.
    private var sortMenu: some View {
        Menu(content: {
            if sortOptions.count > 1 {
                Picker("Trier par", selection: $filters.sortBy) {
                    ForEach(sortOptions, id: \.self) { field in
                        Text(field.label).tag(field)
                    }
                }
            }
            Picker("Ordre", selection: $filters.sortDir) {
                ForEach([SortDirection.desc, .asc], id: \.self) { direction in
                    Label(direction.label, systemImage: direction.icon).tag(direction)
                }
            }
        }, label: {
            chipLabel(filters.sortBy.label, systemImage: filters.sortDir.icon)
        })
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .tint(Palette.primary)
    }

    private func chipButton(
        title: String,
        systemImage: String,
        isActive: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action, label: {
            chipLabel(title, systemImage: systemImage)
        })
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .tint(isActive ? Palette.primary : Color.secondary)
    }

    private func chipLabel(_ title: String, systemImage: String) -> some View {
        Label(title, systemImage: systemImage)
            .font(.subheadline)
            .fontWeight(.medium)
    }
}

#Preview {
    CollectionFilterRail(
        filters: .constant(CollectionFilters()),
        sortOptions: SortField.collectionOptions,
        onFilterTap: {}
    )
    .padding()
}

#Preview("Filtres actifs") {
    CollectionFilterRail(
        filters: .constant(CollectionFilters(rarities: [.R, .M], sets: ["MH3"])),
        sortOptions: SortField.collectionOptions,
        onFilterTap: {}
    )
    .padding()
}
