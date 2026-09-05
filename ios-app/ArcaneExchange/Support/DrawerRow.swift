import SwiftUI

/// A list row that opens a sub-drawer — the mockup's `IRow` with a chevron: icon and title on
/// the left, the current value on the right. Used by Réglages and the collection's filters.
struct DrawerRow: View {
    let title: String
    let systemImage: String
    let value: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Label(title, systemImage: systemImage)
                    .foregroundStyle(.primary)
                Spacer(minLength: 8)
                Text(value)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                Image(systemName: "chevron.right")
                    .font(.footnote)
                    .fontWeight(.semibold)
                    .foregroundStyle(.tertiary)
            }
            // Inside the label on purpose: a plain button only hit-tests what its label draws,
            // so without this the gap between the title and the chevron swallows the tap.
            .contentShape(.rect)
        }
        // Without this the row reads as one big tinted link instead of a list row.
        .buttonStyle(.plain)
    }
}

#Preview {
    List {
        DrawerRow(title: "Sets", systemImage: "square.stack.3d.up", value: "2 sur 14") {}
        DrawerRow(title: "Raretés", systemImage: "line.3.horizontal.decrease", value: "Toutes") {}
    }
}
