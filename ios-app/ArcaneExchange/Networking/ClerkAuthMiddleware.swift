import ClerkKit
import Foundation
import HTTPTypes
import OpenAPIRuntime

/// Injects a fresh Clerk session token as a Bearer token on every request.
///
/// The token is never cached locally — `Clerk.shared.auth.getToken()` is called
/// on every request, mirroring `frontend-vue/app/composables/useApi.ts`, and Clerk's
/// SDK owns refresh/caching internally. When there is no active session (e.g. calling
/// a public endpoint before sign-in), the request is forwarded without a header.
struct ClerkAuthMiddleware: ClientMiddleware {
    func intercept(
        _ request: HTTPRequest,
        body: HTTPBody?,
        baseURL: URL,
        operationID _: String,
        next: @Sendable (HTTPRequest, HTTPBody?, URL) async throws -> (HTTPResponse, HTTPBody?)
    ) async throws -> (HTTPResponse, HTTPBody?) {
        var request = request
        if let token = try? await Clerk.shared.auth.getToken() {
            request.headerFields[.authorization] = "Bearer \(token)"
        }
        return try await next(request, body, baseURL)
    }
}
