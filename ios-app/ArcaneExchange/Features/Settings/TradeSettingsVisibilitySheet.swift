import SwiftUI

/// Sub-drawer "Visibilité de la collection" (the mockup's `sheet === 'vis'`): the three modes,
/// each with what it actually exposes to other players.
struct VisibilitySheet: View {
    let model: AccountSettingsViewModel

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                ForEach(CollectionVisibility.ordered, id: \.self) { option in
                    Button(action: { select(option) }, label: {
                        HStack(alignment: .top, spacing: 12) {
                            VStack(alignment: .leading, spacing: 3) {
                                Text(option.label)
                                    .foregroundStyle(.primary)
                                Text(option.detail)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer(minLength: 8)
                            if model.visibility == option {
                                Image(systemName: "checkmark")
                                    .fontWeight(.semibold)
                                    .foregroundStyle(.tint)
                            }
                        }
                        // Inside the label on purpose: a plain button only hit-tests what its
                        // label draws, so without this the empty space swallows the tap.
                        .contentShape(.rect)
                    })
                    // Without this the row reads as one big tinted link instead of a list row.
                    .buttonStyle(.plain)
                }
                .disabled(model.isSavingVisibility)
            }
            .navigationTitle("Visibilité")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    CloseButton { dismiss() }
                }
            }
            .writeErrorAlert(model)
        }
        .presentationDetents([.medium])
    }

    private func select(_ option: CollectionVisibility) {
        Task { await model.setVisibility(option) }
    }
}
