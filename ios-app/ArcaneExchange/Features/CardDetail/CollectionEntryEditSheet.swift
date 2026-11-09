import SwiftUI

/// Edits one entry of a card: its quantity and finish, or removes it. A refusal (a quantity under
/// what an accepted trade committed, say) stays in the model's `writeError`, shown as an alert.
struct CollectionEntryEditSheet: View {
    let entry: CollectionEntryDetail
    let model: CollectionEntriesViewModel

    @Environment(\.dismiss) private var dismiss
    @State private var quantity: Int
    @State private var foil: Bool

    init(entry: CollectionEntryDetail, model: CollectionEntriesViewModel) {
        self.entry = entry
        self.model = model
        _quantity = State(initialValue: Int(entry.quantity))
        _foil = State(initialValue: entry.foil)
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
                } footer: {
                    if let binder = entry.binder {
                        Text("Binder : \(binder.name)")
                    }
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
            .navigationTitle("Exemplaire")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Enregistrer") {
                        Task {
                            if await model.edit(entry, quantity: quantity, foil: foil) {
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
        .presentationDetents([.medium])
    }
}
