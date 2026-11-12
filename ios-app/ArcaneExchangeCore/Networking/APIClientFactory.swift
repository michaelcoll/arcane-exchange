import APIClient
import ClerkKit
import Foundation
import HTTPTypes
import OpenAPIRuntime
import OpenAPIURLSession

/// Builds the live API client, the only way the app (and the widgets) reach the API (ADR 0020).
/// Callers hold it as `any APIProtocol`, read the success of an `Output` through its generated
/// accessor (`.ok`, `.noContent`…) and catch failures with `APIError.read`.
package enum APIClientFactory {
    /// A client for the signed-in player, built once and kept for the whole session.
    ///
    /// A 401 calls `onUnauthorized` (once, until `latch` is re-armed when the player signs in
    /// again) then raises the `unauthorized` error. The app passes the Clerk sign-out; the widgets, which share the
    /// app's session, pass an empty closure and never sign anyone out.
    ///
    /// Any response to a mutation listed in the operation table of the widgets reload middleware
    /// (an error included: a 409 means the trade moved on) calls `onWidgetsChanged` with the widget
    /// to reload. Only the app passes `HomeWidgetReload.reload`; the default does nothing.
    package static func authenticated(
        onUnauthorized: @escaping @Sendable () async -> Void,
        onWidgetsChanged: @escaping @Sendable (HomeWidgetKind) -> Void = { _ in },
        latch: UnauthorizedLatch = UnauthorizedLatch(),
        transport: any ClientTransport = URLSessionTransport(),
        token: @escaping @Sendable () async -> String? = { try? await Clerk.shared.auth.getToken() },
        baseURL: @escaping @Sendable () -> URL = { AppConfig.apiBaseURL() }
    ) -> any APIProtocol {
        client(
            transport: transport,
            baseURL: baseURL,
            middlewares: [
                ClerkAuthMiddleware(token: token),
                APIErrorMiddleware(onUnauthorized: onUnauthorized, latch: latch),
                // Inside the error middleware, so it sees the response before it becomes an `APIError`.
                WidgetsReloadMiddleware(onWidgetsChanged: onWidgetsChanged)
            ]
        )
    }

    /// A client for the public endpoints (`get_stats`, `get_showcase`): no token, and a 401
    /// signs nobody out.
    package static func anonymous(
        transport: any ClientTransport = URLSessionTransport(),
        baseURL: @escaping @Sendable () -> URL = { AppConfig.apiBaseURL() }
    ) -> any APIProtocol {
        client(
            transport: transport,
            baseURL: baseURL,
            middlewares: [APIErrorMiddleware(onUnauthorized: nil, latch: UnauthorizedLatch())]
        )
    }

    /// The authenticated client of previews and environment defaults: it never signs anyone out.
    package static let preview = authenticated(onUnauthorized: {})

    private static func client(
        transport: any ClientTransport,
        baseURL: @escaping @Sendable () -> URL,
        middlewares: [any ClientMiddleware]
    ) -> APIClient.Client {
        // The base URL is editable from the iOS Settings app while the process is alive: the
        // first middleware swaps in the current one on every request, so this one client
        // never goes stale.
        APIClient.Client(
            serverURL: baseURL(),
            transport: transport,
            middlewares: [CurrentBaseURLMiddleware(baseURL: baseURL)] + middlewares
        )
    }
}

private struct CurrentBaseURLMiddleware: ClientMiddleware {
    let baseURL: @Sendable () -> URL

    func intercept(
        _ request: HTTPRequest,
        body: HTTPBody?,
        baseURL _: URL,
        operationID _: String,
        next: @Sendable (HTTPRequest, HTTPBody?, URL) async throws -> (HTTPResponse, HTTPBody?)
    ) async throws -> (HTTPResponse, HTTPBody?) {
        try await next(request, body, baseURL())
    }
}

/// Asks for the reload of the widget a mutation touches, as soon as the server answered it,
/// whatever the status. A transport failure or a cancellation reloads nothing. An operation
/// absent from the table (the other mutations, every read) reloads nothing.
private struct WidgetsReloadMiddleware: ClientMiddleware {
    /// The widget each operation touches. Rating a trade, the import (it answers 202 before
    /// the collection changes) and the trade settings touch none.
    static let table: [String: HomeWidgetKind] = [
        Operations.create_trade.id: .trades,
        Operations.add_trade_card.id: .trades,
        Operations.remove_trade_card.id: .trades,
        Operations.accept_trade.id: .trades,
        Operations.confirm_trade.id: .trades,
        Operations.abandon_trade.id: .trades,
        Operations.create_binder.id: .collection,
        Operations.rename_binder.id: .collection,
        Operations.delete_binder.id: .collection,
        Operations.add_collection_entry.id: .collection,
        Operations.update_collection_entry.id: .collection,
        Operations.move_collection_entry.id: .collection,
        Operations.remove_collection_entry.id: .collection
    ]

    let onWidgetsChanged: @Sendable (HomeWidgetKind) -> Void

    func intercept(
        _ request: HTTPRequest,
        body: HTTPBody?,
        baseURL: URL,
        operationID: String,
        next: @Sendable (HTTPRequest, HTTPBody?, URL) async throws -> (HTTPResponse, HTTPBody?)
    ) async throws -> (HTTPResponse, HTTPBody?) {
        let result = try await next(request, body, baseURL)
        if let kind = Self.table[operationID] {
            onWidgetsChanged(kind)
        }
        return result
    }
}

/// Raises every failure as an `APIError`: a response of status 400 and above, decoded from its
/// `ErrorResponse` body when it has one, and a transport failure. The generated error cases of
/// an `Output` are never reached.
private struct APIErrorMiddleware: ClientMiddleware {
    private struct ErrorBody: Decodable {
        let code: String?
    }

    /// The most an error body is read up to: anything bigger is not an `ErrorResponse`.
    private static let maxBodySize = 64 * 1024

    let onUnauthorized: (@Sendable () async -> Void)?
    let latch: UnauthorizedLatch

    func intercept(
        _ request: HTTPRequest,
        body: HTTPBody?,
        baseURL: URL,
        operationID _: String,
        next: @Sendable (HTTPRequest, HTTPBody?, URL) async throws -> (HTTPResponse, HTTPBody?)
    ) async throws -> (HTTPResponse, HTTPBody?) {
        let response: HTTPResponse
        let responseBody: HTTPBody?
        do {
            (response, responseBody) = try await next(request, body, baseURL)
        } catch is CancellationError {
            throw CancellationError()
        } catch let error as URLError where error.code == .cancelled {
            throw error
        } catch {
            throw APIError.read(error)
        }

        guard response.status.code >= 400 else {
            return (response, responseBody)
        }

        let status = response.status.code
        var code = await Self.code(in: responseBody)
        if status == 401 {
            code = code ?? "unauthorized"
            if let onUnauthorized, await latch.trip() {
                await onUnauthorized()
            }
        }
        throw APIError(code: code, status: status)
    }

    private static func code(in body: HTTPBody?) async -> String? {
        guard let body,
              let bytes = try? await ArraySlice<UInt8>(collecting: body, upTo: maxBodySize)
        else { return nil }
        return (try? JSONDecoder().decode(ErrorBody.self, from: Data(bytes)))?.code
    }
}

/// Lets one 401 sign the player out, however many requests fail at the same time or later: it
/// stays tripped until `rearm()`, which the app calls when the player signs in again.
package actor UnauthorizedLatch {
    private var isTripped = false

    package init() {}

    /// `true` for the first caller only.
    package func trip() -> Bool {
        defer { isTripped = true }
        return !isTripped
    }

    package func rearm() {
        isTripped = false
    }
}
