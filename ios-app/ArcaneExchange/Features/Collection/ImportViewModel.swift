import APIClient
import Foundation
import OpenAPIRuntime

/// Backs the ManaBox CSV import sheet: submits the file, then polls `GET
/// /collection/import/{id}` every second until the import reaches a terminal state. A `409`
/// (an import is already running for this user) switches to following that import instead of
/// failing outright — mirrors `frontend-vue/app/composables/useCardImportFlow.ts`.
@MainActor
@Observable
final class ImportViewModel {
    enum LoadError: Equatable {
        case unauthorized
        /// A 409 with no active import found when listing — should not normally happen (the
        /// active import that caused the 409 has since finished), but the user is told plainly.
        case conflict
        case network
        case http(Int)
        case unexpected

        var message: String {
            switch self {
            case .unauthorized:
                "Session expirée. Reconnecte-toi pour importer ta collection."
            case .conflict:
                "Un import est déjà en cours pour ton compte."
            case .network:
                "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."
            case let .http(status):
                "Le serveur a répondu \(status)."
            case .unexpected:
                "Réponse inattendue du serveur."
            }
        }
    }

    enum Step: Equatable {
        case pick
        case progress
        case done
    }

    private(set) var step: Step = .pick
    private(set) var status: Components.Schemas.CardImportResponse?
    private(set) var loadError: LoadError?
    private(set) var wasAlreadyRunning = false

    private var pollTask: Task<Void, Never>?

    var progressFraction: Double {
        guard let status, status.total_lines > 0 else {
            return status?.status == "completed" ? 1 : 0
        }
        return Double(status.processed_lines) / Double(status.total_lines)
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
            switch try await postImport(csv: csv) {
            case let .started(id):
                poll(id: id)
            case .conflict:
                wasAlreadyRunning = true
                await followActiveImport()
            }
        } catch let error as APIClientError {
            switch error {
            case .unauthorized: loadError = .unauthorized
            case let .undocumented(statusCode): loadError = .http(statusCode)
            }
        } catch {
            loadError = Self.networkOrUnexpected(error)
        }
    }

    private enum StartOutcome {
        case started(id: String)
        case conflict
    }

    private func followActiveImport() async {
        let imports: [Components.Schemas.CardImportResponse]
        do {
            imports = try await listImports()
        } catch {
            loadError = Self.networkOrUnexpected(error)
            return
        }
        guard let active = imports.first(where: { $0.status == "pending" || $0.status == "running" })
        else {
            loadError = .conflict
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
                    let current = try await fetchImport(id: id)
                    status = current
                    if current.status == "completed" || current.status == "failed" {
                        step = .done
                        return
                    }
                } catch {
                    loadError = Self.networkOrUnexpected(error)
                    step = .pick
                    return
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    private func postImport(csv: String) async throws -> StartOutcome {
        let body = OpenAPIRuntime.HTTPBody(csv)
        switch try await APIClientProvider.shared.import_cards(.init(body: .plainText(body))) {
        case let .accepted(response):
            return try .started(id: response.body.json.id)
        case .conflict:
            return .conflict
        case .badRequest:
            throw APIClientError.undocumented(statusCode: 400)
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func fetchImport(id: String) async throws -> Components.Schemas.CardImportResponse {
        switch try await APIClientProvider.shared.get_card_import(.init(path: .init(id: id))) {
        case let .ok(response):
            return try response.body.json
        case .unauthorized:
            throw APIClientError.unauthorized
        case .notFound:
            throw APIClientError.undocumented(statusCode: 404)
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private func listImports() async throws -> [Components.Schemas.CardImportResponse] {
        switch try await APIClientProvider.shared.list_card_imports(.init()) {
        case let .ok(response):
            return try response.body.json
        case .unauthorized:
            throw APIClientError.unauthorized
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private static func networkOrUnexpected(_ error: Error) -> LoadError {
        if let apiError = error as? APIClientError {
            switch apiError {
            case .unauthorized: return .unauthorized
            case let .undocumented(statusCode): return .http(statusCode)
            }
        }
        let underlying = (error as? ClientError)?.underlyingError ?? error
        return underlying is URLError ? .network : .unexpected
    }
}
