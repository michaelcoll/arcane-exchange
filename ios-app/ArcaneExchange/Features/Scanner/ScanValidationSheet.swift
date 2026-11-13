import ArcaneExchangeCore
import SwiftUI

/// The sheet that ends a scan: how many cards go in (and what they are worth, when known), the
/// binder and the sealed product they join, and the button that sends the batch. A failure stays in
/// the model's `error`, shown as an alert, and leaves the batch as it was.
struct ScanValidationSheet: View {
    let model: ScanValidationViewModel
    let cardCount: Int
    /// The value of the batch in cents, when the prices are known.
    var valueCents: Int?
    /// Called once the batch is validated and emptied.
    let onValidated: () -> Void
    /// Called when the sheet closes without a validation, the lines the catalog refused being
    /// flagged in the list: the player goes back to correct them.
    let onRefused: () -> Void

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        @Bindable var model = model
        NavigationStack {
            Form {
                Section {
                    LabeledContent("Cartes", value: "\(cardCount)")
                    if let valueCents {
                        LabeledContent("Valeur", value: Price.euros(cents: valueCents))
                    }
                }
                Section {
                    Picker("Binder", selection: $model.binderID) {
                        Text(BinderCopy.noBinder).tag(String?.none)
                        ForEach(model.binders, id: \.id) { binder in
                            Text(binder.name).tag(String?.some(binder.id))
                        }
                    }
                    Picker("Produit scellé", selection: $model.sealedProductID) {
                        Text("Aucun").tag(String?.none)
                        ForEach(model.sealedProducts, id: \.id) { product in
                            Text(PurchaseCopy.choice(of: product)).tag(String?.some(product.id))
                        }
                    }
                } footer: {
                    if let loadError = model.loadError {
                        Text(loadError)
                    } else if model.sealedProductID == nil {
                        Text("Les doublons déjà en collection sont additionnés.")
                    } else {
                        Text("Les cartes rejoignent le contenu du produit, sans prix d'achat.")
                    }
                }
            }
            .disabled(model.isSending)
            .navigationTitle("Ajouter à la collection")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(ScanBatchCopy.add(cards: cardCount)) {
                        Task { await send() }
                    }
                    .disabled(!model.canSend)
                }
            }
            .alert(
                "Ajout impossible",
                isPresented: Binding(get: { model.error != nil }, set: {
                    if !$0 {
                        let refused = !model.rejectedLineIDs.isEmpty
                        model.error = nil
                        if refused {
                            onRefused()
                            dismiss()
                        }
                    }
                }),
                actions: { Button("OK", role: .cancel) {} },
                message: { Text(model.error ?? "") }
            )
        }
        .presentationDetents([.medium, .large])
        .task { await model.load() }
    }

    private func send() async {
        if await model.send() {
            onValidated()
            dismiss()
        }
    }
}
