import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The Achats screen, opened from the Collection tab: the player's purchases, most recent
/// first, a form to record a sealed product, and the edition and deletion of a purchase.
/// Deleting one never removes a card from the collection.
struct PurchasesView: View {
    /// The form on screen: a creation, or the edition of one purchase.
    private enum FormTarget: Identifiable {
        case create
        case edit(Purchase)

        var id: String {
            switch self {
            case .create: "create"
            case let .edit(purchase): "edit-\(purchase.id)"
            }
        }
    }

    private let api: any APIProtocol
    @State private var model: PurchasesViewModel
    @State private var form: FormTarget?
    @State private var purchaseToDelete: Purchase?

    @Environment(\.dismiss) private var dismiss

    init(api: any APIProtocol) {
        self.api = api
        _model = State(initialValue: PurchasesViewModel(api: api))
    }

    var body: some View {
        NavigationStack {
            content
                .navigationTitle("Achats")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarLeading) {
                        Button("Nouvel achat", systemImage: "plus") { form = .create }
                            .disabled(model.isWriting)
                    }
                    ToolbarItem(placement: .topBarTrailing) {
                        CloseButton { dismiss() }
                    }
                }
                .task { await model.reload() }
                .sheet(item: $form) { target in
                    PurchaseFormView(api: api, editing: editedPurchase(in: target)) {
                        Task { await model.reload() }
                    }
                }
                .confirmationDialog(
                    "Supprimer cet achat ?",
                    isPresented: Binding(get: { purchaseToDelete != nil }, set: {
                        if !$0 {
                            purchaseToDelete = nil
                        }
                    }),
                    titleVisibility: .visible,
                    presenting: purchaseToDelete
                ) { purchase in
                    Button("Supprimer « \(PurchaseCopy.title(of: purchase)) »", role: .destructive) {
                        Task { await model.delete(purchase) }
                    }
                    Button("Annuler", role: .cancel) {}
                } message: { _ in
                    Text("Les cartes de ta collection ne sont pas retirées.")
                }
                .alert(
                    "Suppression impossible",
                    isPresented: Binding(
                        get: { model.writeError != nil },
                        set: {
                            if !$0 {
                                model.writeError = nil
                            }
                        }
                    ),
                    presenting: model.writeError,
                    actions: { _ in Button("OK", role: .cancel) {} },
                    message: { Text($0) }
                )
        }
        .presentationDetents([.large])
    }

    private func editedPurchase(in target: FormTarget) -> Purchase? {
        if case let .edit(purchase) = target {
            purchase
        } else {
            nil
        }
    }

    @ViewBuilder private var content: some View {
        switch model.paginator.phase {
        case .firstLoad:
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(message):
            ContentUnavailableView {
                Label("Achats indisponibles", systemImage: "exclamationmark.triangle")
            } description: {
                Text(message)
            } actions: {
                Button("Réessayer") { Task { await model.reload() } }
            }
        case .empty:
            ContentUnavailableView(
                "Aucun achat",
                systemImage: "bag",
                description: Text("Enregistre un booster, une display ou un deck pour suivre ce que tu dépenses.")
            )
        case .content:
            List(model.paginator.items, id: \.id) { purchase in
                NavigationLink {
                    PurchaseDetailView(purchase: purchase, api: api)
                } label: {
                    row(purchase)
                }
                .task { await model.paginator.loadMore(after: purchase) }
            }
            .refreshable { await model.reload() }
        }
    }

    private func row(_ purchase: Purchase) -> some View {
        HStack(spacing: 12) {
            if let set = PurchaseCopy.set(of: purchase) {
                SetSymbol(setCode: set.code)
                    .foregroundStyle(.secondary)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(PurchaseCopy.title(of: purchase))
                Text(PurchaseCopy.subtitle(of: purchase))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Text(Price.euros(cents: Int(purchase.amount_cents)))
                .font(.body.monospacedDigit())
        }
        .swipeActions(edge: .trailing) {
            Button("Supprimer", systemImage: "trash", role: .destructive) { purchaseToDelete = purchase }
            Button("Modifier", systemImage: "pencil") { form = .edit(purchase) }
                .tint(Palette.primary)
        }
    }
}
