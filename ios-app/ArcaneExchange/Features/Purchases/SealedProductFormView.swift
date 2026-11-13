import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The form that records a sealed product, or edits one: type, set among the known sets, optional
/// name, price in euros and day of the purchase (today for a new one).
struct SealedProductFormView: View {
    @State private var model: SealedProductFormViewModel
    private let onSaved: () -> Void

    @Environment(\.dismiss) private var dismiss

    init(api: any APIProtocol, editing purchase: Purchase?, onSaved: @escaping () -> Void) {
        _model = State(initialValue: SealedProductFormViewModel(api: api, editing: purchase))
        self.onSaved = onSaved
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Picker("Type", selection: $model.productType) {
                        ForEach(SealedProductType.allCases, id: \.self) { type in
                            Text(PurchaseCopy.label(for: type)).tag(type)
                        }
                    }
                    NavigationLink {
                        SealedProductSetPicker(model: model)
                    } label: {
                        LabeledContent("Set", value: model.selectedSet?.name ?? "Choisir")
                    }
                    TextField("Nom (facultatif)", text: $model.name)
                }
                Section {
                    LabeledContent("Prix") {
                        HStack(spacing: 4) {
                            TextField("0", text: $model.priceText)
                                .keyboardType(.decimalPad)
                                .multilineTextAlignment(.trailing)
                            Text("€").foregroundStyle(.secondary)
                        }
                    }
                    DatePicker("Date", selection: $model.date, in: ...Date(), displayedComponents: .date)
                }
                if let error = model.writeError {
                    Section {
                        Text(error).foregroundStyle(Palette.down)
                    }
                }
            }
            .navigationTitle(model.isEditing ? "Modifier l'achat" : "Nouvel achat")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Enregistrer") { save() }
                        .disabled(!model.canSubmit || model.isSaving)
                }
            }
            .task { await model.loadSets() }
        }
        .presentationDetents([.large])
    }

    private func save() {
        Task {
            if await model.submit() {
                onSaved()
                dismiss()
            }
        }
    }
}

/// The searchable list of sets a sealed product can belong to.
private struct SealedProductSetPicker: View {
    let model: SealedProductFormViewModel

    @Environment(\.dismiss) private var dismiss
    @State private var query = ""

    var body: some View {
        List(SetSearch.filter(model.sets, matching: query), id: \.code) { set in
            CheckRow(
                title: set.name,
                isSelected: model.setCode == set.code,
                accessory: { SetSymbol(setCode: set.code).foregroundStyle(.secondary) },
                action: {
                    model.setCode = set.code
                    dismiss()
                }
            )
        }
        .overlay {
            if let error = model.loadError {
                ContentUnavailableView(
                    "Sets indisponibles",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            }
        }
        .searchable(text: $query, prompt: "Rechercher un set")
        .navigationTitle("Set")
        .navigationBarTitleDisplayMode(.inline)
    }
}
