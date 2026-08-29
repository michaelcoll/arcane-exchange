import APIClient
import Foundation
import OpenAPIRuntime
import OpenAPIURLSession

enum APIClientProvider {
    /// A client bound to the *current* value of `AppConfig.apiBaseURL`.
    ///
    /// Rebuilt on every access on purpose: the base URL is editable from the iOS Settings app
    /// while the process is alive, and a cached client would keep hitting the old server until
    /// the next launch. `Client` only stores its transport and middlewares, and
    /// `URLSessionTransport` defaults to `URLSession.shared`, so this allocates nothing
    /// meaningful — no connection pool is thrown away.
    static var shared: Client {
        Client(
            serverURL: AppConfig.apiBaseURL,
            transport: URLSessionTransport(),
            middlewares: [ClerkAuthMiddleware()]
        )
    }
}

/// Errors surfaced by the generated client for status codes that have no
/// typed response schema in `doc/openapi.yml` (400/401/404 across most operations).
///
/// Each generated operation has its own `Output` enum with an
/// `.undocumented(statusCode: Int, UndocumentedPayload)` case for these — there is
/// no shared type to extend generically, so call sites switch on that case per
/// operation and map a 401 to `.unauthorized` (e.g. to trigger a Clerk sign-out).
enum APIClientError: Error {
    case unauthorized
    case undocumented(statusCode: Int)
}
