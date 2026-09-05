import SwiftUI

/// A multiple-choice list row: an accessory (a set code, a rarity symbol, nothing), the label,
/// and a checkmark when the value is picked. The collection's filter drawers are built out of
/// these.
struct CheckRow<Accessory: View>: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void
    private let accessory: Accessory

    init(
        title: String,
        isSelected: Bool,
        @ViewBuilder accessory: () -> Accessory,
        action: @escaping () -> Void
    ) {
        self.title = title
        self.isSelected = isSelected
        self.action = action
        self.accessory = accessory()
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                accessory
                Text(title)
                    .foregroundStyle(.primary)
                Spacer(minLength: 8)
                if isSelected {
                    Image(systemName: "checkmark")
                        .fontWeight(.semibold)
                        .foregroundStyle(.tint)
                }
            }
            // Inside the label on purpose: a plain button only hit-tests what its label draws,
            // so without this the empty space right of the title swallows the tap.
            .contentShape(.rect)
        }
        // Without this the row reads as one big tinted link instead of a list row.
        .buttonStyle(.plain)
    }
}

extension CheckRow where Accessory == EmptyView {
    init(title: String, isSelected: Bool, action: @escaping () -> Void) {
        self.init(title: title, isSelected: isSelected, accessory: { EmptyView() }, action: action)
    }
}

/// A set code as `CheckRow`'s accessory: monospaced and tinted, wide enough that the titles
/// next to it line up whatever the code's length.
struct CodeBadge: View {
    let code: String

    var body: some View {
        Text(code)
            .font(.caption)
            .fontWeight(.semibold)
            .monospaced()
            .foregroundStyle(.tint)
            .frame(minWidth: 42, alignment: .leading)
    }
}

#Preview {
    List {
        CheckRow(
            title: "Modern Horizons 3",
            isSelected: true,
            accessory: { CodeBadge(code: "MH3") },
            action: {}
        )
        CheckRow(
            title: "Rares",
            isSelected: false,
            accessory: { RaritySymbol(rarity: .R) },
            action: {}
        )
        CheckRow(title: "Sans accessoire", isSelected: false) {}
    }
}
