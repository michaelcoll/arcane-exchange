import SwiftUI
import UniformTypeIdentifiers

/// Sheet for importing a ManaBox collection export: pick a `.csv`, follow the async import's
/// progress, then show the compte-rendu (cards imported, line errors).
struct ImportView: View {
    @State private var model = ImportViewModel()
    @State private var isPickingFile = false
    @State private var pickError: String?

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            content
                .navigationTitle("Importer depuis ManaBox")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        CloseButton { dismiss() }
                    }
                }
                .fileImporter(
                    isPresented: $isPickingFile,
                    allowedContentTypes: [.commaSeparatedText],
                    onCompletion: handlePick
                )
        }
        .onAppear { model.reset() }
        .onDisappear { model.reset() }
    }

    @ViewBuilder private var content: some View {
        switch model.step {
        case .pick:
            pickView
        case .progress:
            progressView
        case .done:
            doneView
        }
    }

    private var pickView: some View {
        VStack(spacing: 16) {
            Image(systemName: "square.and.arrow.down")
                .font(.system(size: 40))
                .foregroundStyle(.tint)
            Text("Exporte ta collection en .csv depuis ManaBox, puis sélectionne-la ici.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)

            Button("Choisir un fichier") { isPickingFile = true }
                .buttonStyle(.borderedProminent)

            if let error = model.loadError {
                Text(error.message)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
            if let pickError {
                Text(pickError)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var progressView: some View {
        VStack(spacing: 16) {
            if model.wasAlreadyRunning {
                Text("Un import est déjà en cours pour ton compte — voici sa progression :")
                    .font(.footnote)
                    .foregroundStyle(.orange)
                    .multilineTextAlignment(.center)
            }

            ProgressView(value: model.progressFraction)
                .progressViewStyle(.linear)

            Text("\(model.status?.processed_lines ?? 0) / \(model.status?.total_lines ?? 0) cartes")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder private var doneView: some View {
        if model.status?.status == "completed" {
            completedView
        } else {
            failedView
        }
    }

    private var completedView: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 40))
                .foregroundStyle(.green)

            if let status = model.status {
                Text("\(status.total_lines) carte(s) importée(s)")
                    .font(.headline)

                if status.error_count > 0 {
                    Text("\(status.error_count) ligne(s) ignorée(s)")
                        .font(.subheadline)
                        .foregroundStyle(.orange)
                }

                if !status.errors.isEmpty {
                    List(status.errors, id: \.line) { lineError in
                        Text("Ligne \(lineError.line) : champ « \(lineError.field) » invalide (\(lineError.value))")
                            .font(.footnote)
                    }
                    .frame(maxHeight: 200)
                    .listStyle(.plain)
                }
            }

            Button("Fermer") { dismiss() }
                .buttonStyle(.borderedProminent)
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var failedView: some View {
        VStack(spacing: 16) {
            Image(systemName: "xmark.circle.fill")
                .font(.system(size: 40))
                .foregroundStyle(.red)

            Text(model.status?.error_message ?? "L'import a échoué.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)

            Button("Réessayer") { model.reset() }
                .buttonStyle(.borderedProminent)
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func handlePick(_ result: Result<URL, Error>) {
        pickError = nil
        guard case let .success(url) = result else {
            pickError = "Impossible de lire le fichier sélectionné."
            return
        }

        // `.fileImporter` hands back a security-scoped URL: reading it without bracketing the
        // access call fails silently on a real device (it only "happens to work" in the
        // simulator, which does not enforce the sandbox the same way).
        guard url.startAccessingSecurityScopedResource() else {
            pickError = "Impossible d'accéder au fichier sélectionné."
            return
        }
        defer { url.stopAccessingSecurityScopedResource() }

        do {
            let csv = try String(contentsOf: url, encoding: .utf8)
            Task { await model.start(csv: csv) }
        } catch {
            pickError = "Le fichier n'est pas lisible en UTF-8."
        }
    }
}

#Preview {
    ImportView()
}
