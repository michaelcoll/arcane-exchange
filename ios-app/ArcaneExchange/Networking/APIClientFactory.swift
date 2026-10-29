import APIClient
import ClerkKit
import Foundation
import HTTPTypes
import OpenAPIRuntime
import OpenAPIURLSession

/// Builds the live API client, the only way the app (and the widgets) reach the API (ADR 0020).
/// Callers hold it as `any APIProtocol`, read the success of an `Output` through its generated
/// accessor (`.ok`, `.noContent`…) and catch failures with `APIError.read`.
enum APIClientFactory {
    /// A client for the signed-in player, built once and kept for the whole session.
    ///
    /// A 401 calls `onUnauthorized` (once, until `latch` is re-armed when the player signs in
    /// again) then raises the `unauthorized` error. The app passes the Clerk sign-out; the widgets, which share the
    /// app's session, pass an empty closure and never sign anyone out.
    static func authenticated(
        onUnauthorized: @escaping @Sendable () async -> Void,
        latch: UnauthorizedLatch = UnauthorizedLatch(),
        transport: any ClientTransport = URLSessionTransport(),
        token: @escaping @Sendable () async -> String? = { try? await Clerk.shared.auth.getToken() },
        baseURL: @escaping @Sendable () -> URL = { AppConfig.apiBaseURL }
    ) -> any APIProtocol {
        client(
            transport: transport,
            baseURL: baseURL,
            middlewares: [
                ClerkAuthMiddleware(token: token),
                APIErrorMiddleware(onUnauthorized: onUnauthorized, latch: latch)
            ]
        )
    }

    /// A client for the public endpoints (`get_stats`, `get_showcase`): no token, and a 401
    /// signs nobody out.
    static func anonymous(
        transport: any ClientTransport = URLSessionTransport(),
        baseURL: @escaping @Sendable () -> URL = { AppConfig.apiBaseURL }
    ) -> any APIProtocol {
        client(
            transport: transport,
            baseURL: baseURL,
            middlewares: [APIErrorMiddleware(onUnauthorized: nil, latch: UnauthorizedLatch())]
        )
    }

    /// The authenticated client of previews and environment defaults: it never signs anyone out.
    static let preview = authenticated(onUnauthorized: {})

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
actor UnauthorizedLatch {
    private var isTripped = false

    /// `true` for the first caller only.
    func trip() -> Bool {
        defer { isTripped = true }
        return !isTripped
    }

    func rearm() {
        isTripped = false
    }
}
