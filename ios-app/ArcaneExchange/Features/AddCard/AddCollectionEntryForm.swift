import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The last step of adding a card: the language, finish, quantity and binder of the copies. A
/// refusal (a binder gone, say) stays in the model's `writeError`, shown as an alert.
struct AddCollectionEntryForm: View {
    @State private var model: AddCollectionEntryViewModel
    private let onAdded: () -> Void

    init(card: CatalogCard, api: any APIProtocol, onAdded: @escaping () -> Void) {
        _model = State(initialValue: AddCollectionEntryViewModel(card: card, api: api))
        self.onAdded = onAdded
    }

    var body: some View {
        Form {
            Section {
                HStack(spacing: 12) {
                    CatalogCardImage(card: model.card)
                        .frame(width: 72)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(model.card.french_name ?? model.card.name)
                            .font(.headline)
                        CatalogCardSetLine(card: model.card)
                    }
                }
            }
            Section {
                Picker("Langue", selection: $model.language) {
                    ForEach(CardLanguage.allCases) { language in
                        Text(language.label).tag(language)
                    }
                }
                Picker("Finition", selection: $model.foil) {
                    Text("Normale").tag(false)
                    Text("Foil").tag(true)
                }
                Stepper("Quantité : \(model.quantity)", value: $model.quantity, in: 1 ... 255)
                Picker("Binder", selection: $model.binderID) {
                    Text(BinderCopy.noBinder).tag(String?.none)
                    ForEach(model.binders, id: \.id) { binder in
                        Text(binder.name).tag(String?.some(binder.id))
                    }
                }
            }
            Section {
                Picker(
                    "Produit scellé",
                    selection: Binding(get: { model.sealedProductID }, set: { model.chooseSealedProduct($0) })
                ) {
                    Text("Aucun").tag(String?.none)
                    ForEach(model.sealedProducts, id: \.id) { product in
                        Text(PurchaseCopy.choice(of: product)).tag(String?.some(product.id))
                    }
                }
                if model.sealedProductID == nil {
                    LabeledContent("Prix d'achat") {
                        HStack(spacing: 4) {
                            TextField("Aucun", text: $model.priceText)
                                .keyboardType(.decimalPad)
                                .multilineTextAlignment(.trailing)
                            Text("€").foregroundStyle(.secondary)
                        }
                    }
                }
            } footer: {
                if let loadError = model.loadError {
                    Text(loadError)
                } else if model.sealedProductID == nil {
                    Text("Par exemplaire. Renseigné, il enregistre un achat à l'unité daté d'aujourd'hui.")
                } else {
                    Text("Les cartes rejoignent le contenu du produit, sans prix d'achat.")
                }
            }
        }
        .disabled(model.isWriting)
        .task {
            await model.loadBinders()
            await model.loadSealedProducts()
        }
        .navigationTitle("Exemplaire")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button("Ajouter") {
                    Task {
                        if await model.add() {
                            onAdded()
                        }
                    }
                }
                .disabled(model.isWriting || !model.canSubmit)
            }
        }
        .alert(
            "Ajout impossible",
            isPresented: Binding(get: { model.writeError != nil }, set: {
                if !$0 {
                    model.writeError = nil
                }
            }),
            actions: { Button("OK", role: .cancel) { model.writeError = nil } },
            message: { Text(model.writeError ?? "") }
        )
    }
}

#Preview {
    NavigationStack {
        AddCollectionEntryForm(card: CatalogCard.previewCatalog[0], api: APIClientFactory.preview, onAdded: {})
    }
}
