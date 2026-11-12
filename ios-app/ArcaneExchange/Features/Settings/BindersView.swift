import APIClient
import ArcaneCore
import SwiftUI

/// The Binders screen, opened from Réglages: the player's binders with their size, a form to
/// create one, to rename one, and the deletion of an empty one. Opening a binder to trade is
/// not done here, it stays in the trade settings.
struct BindersView: View {
    /// The name form: a creation, or the renaming of one binder.
    private enum NameForm: Identifiable {
        case create
        case rename(Binder)

        var id: String {
            switch self {
            case .create: "create"
            case let .rename(binder): "rename-\(binder.id)"
            }
        }

        var title: String {
            switch self {
            case .create: "Nouveau binder"
            case .rename: "Renommer le binder"
            }
        }

        var initialName: String {
            switch self {
            case .create: ""
            case let .rename(binder): binder.name
            }
        }
    }

    @State private var model: BindersViewModel
    @State private var form: NameForm?
    @State private var name = ""
    @State private var binderToDelete: Binder?

    @Environment(\.dismiss) private var dismiss

    init(api: any APIProtocol) {
        _model = State(initialValue: BindersViewModel(api: api))
    }

    var body: some View {
        NavigationStack {
            content
                .navigationTitle("Binders")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarLeading) {
                        Button("Nouveau binder", systemImage: "plus") { open(.create) }
                            .disabled(model.isWriting)
                    }
                    ToolbarItem(placement: .topBarTrailing) {
                        CloseButton { dismiss() }
                    }
                }
                .task { await model.load() }
                .alert(
                    form?.title ?? "",
                    isPresented: Binding(get: { form != nil }, set: {
                        if !$0 {
                            form = nil
                        }
                    }),
                    presenting: form
                ) { form in
                    TextField("Nom", text: $name)
                    Button("Enregistrer") { submit(form) }
                    Button("Annuler", role: .cancel) {}
                }
                .confirmationDialog(
                    "Supprimer ce binder ?",
                    isPresented: Binding(get: { binderToDelete != nil }, set: {
                        if !$0 {
                            binderToDelete = nil
                        }
                    }),
                    titleVisibility: .visible,
                    presenting: binderToDelete
                ) { binder in
                    Button("Supprimer « \(binder.name) »", role: .destructive) {
                        Task { await model.delete(binder) }
                    }
                    Button("Annuler", role: .cancel) {}
                }
                .alert(
                    "Modification impossible",
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
        .presentationDetents([.medium, .large])
    }

    @ViewBuilder
    private var content: some View {
        if let error = model.loadError {
            ContentUnavailableView {
                Label("Binders indisponibles", systemImage: "exclamationmark.triangle")
            } description: {
                Text(error)
            } actions: {
                Button("Réessayer") { Task { await model.load() } }
            }
        } else {
            List {
                if model.binders.isEmpty, !model.isLoading {
                    Text("Aucun binder. Crée-en un pour y ranger tes cartes.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
                ForEach(model.binders, id: \.id) { binder in
                    row(binder)
                }
            }
            .refreshable { await model.load() }
            .overlay {
                if model.isLoading, model.binders.isEmpty {
                    ProgressView().controlSize(.large)
                }
            }
        }
    }

    private func row(_ binder: Binder) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(binder.name)
            Text(BinderCopy.detail(copies: Int(binder.copies), isOpen: binder.is_open))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .swipeActions(edge: .trailing) {
            // Only an empty binder can go: the others would be refused by the server.
            if binder.copies == 0 {
                Button("Supprimer", systemImage: "trash", role: .destructive) { binderToDelete = binder }
            }
            Button("Renommer", systemImage: "pencil") { open(.rename(binder)) }
                .tint(Palette.primary)
        }
    }

    private func open(_ form: NameForm) {
        name = form.initialName
        self.form = form
    }

    private func submit(_ form: NameForm) {
        let submitted = name
        Task {
            switch form {
            case .create: _ = await model.create(name: submitted)
            case let .rename(binder): _ = await model.rename(binder, to: submitted)
            }
        }
    }
}

/// The words of a binder row, kept off the `View`: it is `@MainActor`, and tests call this.
enum BinderCopy {
    /// The choice, and the row label, of copies that sit in no binder.
    static let noBinder = "Sans binder"

    /// "3 exemplaires · ouvert à l'échange", or "Vide" for an empty binder.
    static func detail(copies: Int, isOpen: Bool) -> String {
        let size = copies == 0 ? "Vide" : TradeSettingsCopy.copies(copies)
        return isOpen ? "\(size) · ouvert à l'échange" : size
    }
}

#Preview {
    BindersView(api: APIClientFactory.preview)
}
