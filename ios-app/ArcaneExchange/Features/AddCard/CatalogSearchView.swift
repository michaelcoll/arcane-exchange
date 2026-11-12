import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The addition of a card by hand: search the catalog by English or French name, pick the
/// impression by its image, then fill the form (language, finish, quantity, binder).
struct CatalogSearchView: View {
    @State private var model: CatalogSearchViewModel
    private let api: any APIProtocol
    /// Called once a card was added, before the sheet closes.
    private let onAdded: () -> Void
    @Environment(\.dismiss) private var dismiss

    init(api: any APIProtocol, onAdded: @escaping () -> Void) {
        self.api = api
        self.onAdded = onAdded
        _model = State(initialValue: CatalogSearchViewModel(api: api))
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                setField
                content
            }
            .navigationTitle("Ajouter une carte")
            .navigationBarTitleDisplayMode(.inline)
            .searchable(
                text: $model.query,
                placement: .navigationBarDrawer(displayMode: .always),
                prompt: "Nom en français ou en anglais"
            )
            .task(id: model.searchKey) { await model.search() }
            .navigationDestination(for: CatalogCard.self) { card in
                AddCollectionEntryForm(card: card, api: api) {
                    onAdded()
                    dismiss()
                }
            }
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Fermer") { dismiss() }
                }
            }
        }
    }

    /// The optional restriction to a set, under the name search field.
    private var setField: some View {
        TextField("Set (ex. FDN)", text: $model.setCode)
            .textInputAutocapitalization(.characters)
            .autocorrectionDisabled()
            .textFieldStyle(.roundedBorder)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
    }

    @ViewBuilder private var content: some View {
        if !model.hasQuery {
            ContentUnavailableView(
                "Cherche une carte",
                systemImage: "magnifyingglass",
                description: Text("Entre son nom, en français ou en anglais.")
            )
        } else {
            switch model.paginator.phase {
            case .firstLoad:
                ProgressView()
                    .controlSize(.large)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case let .failed(message):
                ContentUnavailableView {
                    Label("Recherche impossible", systemImage: "exclamationmark.triangle")
                } description: {
                    Text(message)
                } actions: {
                    Button("Réessayer") { Task { await model.paginator.reload() } }
                }
            case .empty:
                ContentUnavailableView.search(text: model.query)
            case .content:
                results
            }
        }
    }

    private var results: some View {
        List {
            ForEach(model.paginator.items, id: \.self) { card in
                NavigationLink(value: card) {
                    CatalogCardRow(card: card)
                }
                .task { await model.paginator.loadMore(after: card) }
            }
            if model.paginator.state == .loadingMore {
                ProgressView().frame(maxWidth: .infinity)
            }
        }
        .listStyle(.plain)
    }
}

#Preview {
    CatalogSearchView(api: APIClientFactory.preview, onAdded: {})
}
