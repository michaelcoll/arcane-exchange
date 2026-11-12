import ArcaneExchangeCore
import SwiftUI

/// Sub-drawer "Binder de réception": where the cards received in a completed Trade arrive,
/// among the player's binders, or the default `trades` binder.
struct TradeSettingsReceivingBinderSheet: View {
    let model: TradeSettingsViewModel

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    option(
                        title: TradeSettingsCopy.defaultReceivingBinder,
                        isSelected: model.receivingBinder == nil,
                        binderID: nil
                    )
                    ForEach(model.binders, id: \.id) { binder in
                        option(
                            title: binder.name,
                            isSelected: model.receivingBinder?.id == binder.id,
                            binderID: binder.id
                        )
                    }
                } footer: {
                    Text(
                        "Les cartes que tu reçois à la fin d'un échange arrivent dans ce binder. "
                            + "Par défaut, elles vont dans un binder nommé « trades », créé à la première réception."
                    )
                }
            }
            .navigationTitle("Binder de réception")
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

    private func option(title: String, isSelected: Bool, binderID: String?) -> some View {
        CheckRow(title: title, isSelected: isSelected) { select(binderID) }
            .disabled(model.isSavingReceivingBinder)
    }

    private func select(_ binderID: String?) {
        Task { await model.setReceivingBinder(binderID) }
    }
}

#Preview {
    TradeSettingsReceivingBinderSheet(model: TradeSettingsViewModel(api: APIClientFactory.preview))
}
