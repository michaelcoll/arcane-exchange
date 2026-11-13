import ArcaneExchangeCore
import SwiftUI

/// Edits one entry of a card: its quantity, finish and purchase price, moves all or some of its copies to another
/// binder (or out of any binder), or removes it. A refusal (a quantity under
/// what an accepted trade committed, say) stays in the model's `writeError`, shown as an alert.
struct CollectionEntryEditSheet: View {
    let entry: CollectionEntryDetail
    let model: CollectionEntriesViewModel

    @Environment(\.dismiss) private var dismiss
    @State private var quantity: Int
    @State private var foil: Bool
    @State private var purchasePrice: String
    @State private var targetBinderID: String?
    @State private var movedQuantity: Int

    init(entry: CollectionEntryDetail, model: CollectionEntriesViewModel) {
        self.entry = entry
        self.model = model
        _quantity = State(initialValue: Int(entry.quantity))
        _foil = State(initialValue: entry.foil)
        _purchasePrice = State(initialValue: entry.purchase_price.map { EuroInput.text(cents: Int($0)) } ?? "")
        _movedQuantity = State(initialValue: Int(entry.quantity))
    }

    /// A move needs a target other than the binder the entry is already in.
    private var canMove: Bool {
        targetBinderID != entry.binder?.id
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Stepper("Quantité : \(quantity)", value: $quantity, in: 1 ... 255)
                    Picker("Finition", selection: $foil) {
                        Text("Normale").tag(false)
                        Text("Foil").tag(true)
                    }
                    HStack {
                        Text("Prix d'achat")
                        TextField("Aucun", text: $purchasePrice)
                            .keyboardType(.decimalPad)
                            .multilineTextAlignment(.trailing)
                        Text("€")
                    }
                } footer: {
                    if let binder = entry.binder {
                        Text("Binder : \(binder.name)")
                    }
                }
                Section("Déplacer") {
                    Picker("Vers", selection: $targetBinderID) {
                        if entry.binder != nil {
                            Text(BinderCopy.noBinder).tag(String?.none)
                        }
                        ForEach(model.binders.filter { $0.id != entry.binder?.id }, id: \.id) { binder in
                            Text(binder.name).tag(String?.some(binder.id))
                        }
                    }
                    Stepper("Quantité : \(movedQuantity)", value: $movedQuantity, in: 1 ... Int(entry.quantity))
                    Button("Déplacer") {
                        Task {
                            if await model.move(entry, to: targetBinderID, quantity: movedQuantity) {
                                dismiss()
                            }
                        }
                    }
                    .disabled(!canMove)
                }
                Section {
                    Button("Retirer de ma collection", role: .destructive) {
                        Task {
                            if await model.remove(entry) {
                                dismiss()
                            }
                        }
                    }
                }
            }
            .disabled(model.isWriting)
            .task { await model.loadBinders() }
            .navigationTitle("Exemplaire")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Enregistrer") {
                        Task {
                            if await model.edit(entry, quantity: quantity, foil: foil, priceText: purchasePrice) {
                                dismiss()
                            }
                        }
                    }
                }
            }
            .alert(
                "Modification impossible",
                isPresented: Binding(get: { model.writeError != nil }, set: {
                    if !$0 {
                        model.writeError = nil
                    }
                }),
                actions: { Button("OK", role: .cancel) { model.writeError = nil } },
                message: { Text(model.writeError ?? "") }
            )
        }
        .presentationDetents([.medium, .large])
    }
}

#Preview {
    CollectionEntryEditSheet(
        entry: .preview,
        model: CollectionEntriesViewModel(card: CollectionCard.previewGrid[0], api: APIClientFactory.preview)
    )
}
