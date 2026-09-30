import SwiftUI

/// Sub-drawer "Classeurs échangeables" (the mockup's `sheet === 'binders'`): which ManaBox
/// binders of the last import are opened to trade.
struct TradeBindersSheet: View {
    let model: AccountSettingsViewModel

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    if model.binders.isEmpty {
                        Text("Aucun classeur dans ton dernier import ManaBox.")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                    ForEach(model.binders, id: \.name) { binder in
                        Toggle(isOn: binding(for: binder.name)) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(binder.name)
                                Text(AccountCopy.cards(Int(binder.card_count)))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .disabled(model.busyBinder != nil)
                    }
                } footer: {
                    Text(
                        "Le nom du classeur vient de ton export ManaBox. Un classeur décoché est "
                            + "invisible pour les autres joueurs, et ses cartes ne partent jamais."
                    )
                }
            }
            .navigationTitle("Classeurs")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    CloseButton { dismiss() }
                }
            }
            .writeErrorAlert(model)
        }
        .presentationDetents([.medium, .large])
    }

    private func binding(for name: String) -> Binding<Bool> {
        Binding(
            get: { model.selectedBinders.contains(name) },
            set: { _ in Task { await model.toggleBinder(name) } }
        )
    }
}
