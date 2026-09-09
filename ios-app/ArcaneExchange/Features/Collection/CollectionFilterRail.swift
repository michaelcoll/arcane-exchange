import SwiftUI

/// Sort menu + filter chip, scrolling with the grid like the mockup's chip rail — the filters
/// are a refinement of the list, not app chrome.
struct CollectionFilterRail: View {
    @Binding var filters: CollectionFilters
    let onFilterTap: () -> Void

    var body: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 8) {
                sortMenu
                chipButton(
                    title: CollectionCopy.filterChip(activeCount: filters.activeCount),
                    systemImage: "line.3.horizontal.decrease",
                    isActive: filters.activeCount > 0
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
            Picker("Trier par", selection: $filters.sortBy) {
                ForEach(SortField.collectionOptions, id: \.self) { field in
                    Text(field.label).tag(field)
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
        .tint(.accentColor)
    }

    private func chipButton(title: String, systemImage: String, isActive: Bool) -> some View {
        Button(action: onFilterTap, label: {
            chipLabel(title, systemImage: systemImage)
        })
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .tint(isActive ? Color.accentColor : Color.secondary)
    }

    private func chipLabel(_ title: String, systemImage: String) -> some View {
        Label(title, systemImage: systemImage)
            .font(.subheadline)
            .fontWeight(.medium)
    }
}

#Preview {
    CollectionFilterRail(filters: .constant(CollectionFilters()), onFilterTap: {})
        .padding()
}

#Preview("Filtres actifs") {
    CollectionFilterRail(
        filters: .constant(CollectionFilters(rarities: [.R, .M], sets: ["MH3"])),
        onFilterTap: {}
    )
    .padding()
}
