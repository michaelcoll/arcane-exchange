import APIClient
import Foundation
import OpenAPIRuntime

/// Backs the ManaBox CSV import sheet: submits the file, then polls `GET
/// /collection/import/{id}` every second until the import reaches a terminal state. A `409`
/// (an import is already running for this user) switches to following that import instead of
/// failing outright — mirrors `frontend/app/composables/useCardImportFlow.ts`.
@MainActor
@Observable
final class ImportViewModel {
    enum Step: Equatable {
        case pick
        case progress
        case done
    }

    private let api: any APIProtocol

    init(api: any APIProtocol) {
        self.api = api
    }

    private(set) var step: Step = .pick
    private(set) var status: Components.Schemas.CardImportResponse?
    /// French message of the last failure, ready to show.
    private(set) var loadError: String?
    private(set) var wasAlreadyRunning = false

    private var pollTask: Task<Void, Never>?

    var progressFraction: Double {
        guard let status, status.total_lines > 0 else {
            return status?.status == "completed" ? 1 : 0
        }
        return Double(status.processed_lines) / Double(status.total_lines)
    }

    /// Why the followed import failed, translated from its `error_code`; `nil` unless it failed.
    var failureMessage: String? {
        guard let status, status.status == "failed" else { return nil }
        return APIErrorMessages.message(for: status.error_code, fallback: "L'import a échoué.")
    }

    /// Cancels any in-flight polling and returns to the picker — called when the sheet is
    /// reopened after a previous run, and when the view disappears.
    func reset() {
        pollTask?.cancel()
        pollTask = nil
        step = .pick
        status = nil
        loadError = nil
        wasAlreadyRunning = false
    }

    func start(csv: String) async {
        loadError = nil
        wasAlreadyRunning = false

        do {
            let body = OpenAPIRuntime.HTTPBody(csv)
            let started = try await api.import_cards(.init(body: .plainText(body))).accepted.body.json
            poll(id: started.id)
        } catch {
            let apiError = APIError.read(error)
            if apiError.code == "import_already_running" {
                wasAlreadyRunning = true
                await followActiveImport()
            } else {
                loadError = apiError.message
            }
        }
    }

    private func followActiveImport() async {
        let imports: [Components.Schemas.CardImportResponse]
        do {
            imports = try await api.list_card_imports(.init()).ok.body.json
        } catch {
            loadError = APIError.read(error).message
            return
        }
        guard let active = imports.first(where: { $0.status == "pending" || $0.status == "running" })
        else {
            loadError = APIErrorMessages.message(for: "import_already_running")
            return
        }
        poll(id: active.id)
    }

    private func poll(id: String) {
        step = .progress
        pollTask?.cancel()
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                do {
                    let current = try await api.get_card_import(.init(path: .init(id: id))).ok.body.json
                    status = current
                    if current.status == "completed" || current.status == "failed" {
                        step = .done
                        // A finished import changes the card counts on the widget.
                        HomeWidgetReload.reload(.collection)
                        return
                    }
                } catch {
                    loadError = APIError.read(error).message
                    step = .pick
                    return
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }
}
