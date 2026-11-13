import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The addition of a card by hand: search the catalog by English or French name, pick the
/// impression by its image, then fill the form (language, finish, quantity, binder).
struct CatalogSearchView: View {
    @State private var model: CatalogSearchViewModel
    /// How a result is used: the form of the manual addition, or a pick handed to the caller (the scan
    /// batch corrections).
    private enum Use {
        case addForm(api: any APIProtocol, onAdded: () -> Void)
        case pick(title: String, onPick: (CatalogCard) -> Void)
    }

    private let use: Use
    @Environment(\.dismiss) private var dismiss

    /// `onAdded` is called once a card was added, before the sheet closes.
    init(api: any APIProtocol, onAdded: @escaping () -> Void) {
        use = .addForm(api: api, onAdded: onAdded)
        _model = State(initialValue: CatalogSearchViewModel(api: api))
    }

    /// Hands the impression picked to `onPick` and closes, with no form.
    init(search: CatalogSearchViewModel, title: String, onPick: @escaping (CatalogCard) -> Void) {
        use = .pick(title: title, onPick: onPick)
        _model = State(initialValue: search)
    }

    private var title: String {
        if case let .pick(title, _) = use {
            title
        } else {
            "Ajouter une carte"
        }
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                setField
                content
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .searchable(
                text: $model.query,
                placement: .navigationBarDrawer(displayMode: .always),
                prompt: "Nom en français ou en anglais"
            )
            .task(id: model.searchKey) { await model.search() }
            .navigationDestination(for: CatalogCard.self) { card in
                if case let .addForm(api, onAdded) = use {
                    AddCollectionEntryForm(card: card, api: api) {
                        onAdded()
                        dismiss()
                    }
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
                row(for: card)
                    .task { await model.paginator.loadMore(after: card) }
            }
            if model.paginator.state == .loadingMore {
                ProgressView().frame(maxWidth: .infinity)
            }
        }
        .listStyle(.plain)
    }

    @ViewBuilder private func row(for card: CatalogCard) -> some View {
        switch use {
        case .addForm:
            NavigationLink(value: card) {
                CatalogCardRow(card: card)
            }
        case let .pick(_, onPick):
            Button {
                onPick(card)
                dismiss()
            } label: {
                CatalogCardRow(card: card)
            }
            .buttonStyle(.plain)
        }
    }
}

#Preview {
    CatalogSearchView(api: APIClientFactory.preview, onAdded: {})
}
