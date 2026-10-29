import APIClient
import Foundation
import HTTPTypes
import OpenAPIRuntime
import Testing

@testable import ArcaneExchange

/// Raw HTTP in memory: what the live client does with a response, without the network.
private final class StubTransport: ClientTransport, @unchecked Sendable {
    typealias Reply = @Sendable (HTTPRequest, URL) async throws -> (HTTPResponse, HTTPBody?)

    private let lock = NSLock()
    private var reply: Reply
    private var sentRequests: [HTTPRequest] = []
    private var sentBaseURLs: [URL] = []

    init(_ reply: @escaping Reply) {
        self.reply = reply
    }

    var requests: [HTTPRequest] {
        lock.withLock { sentRequests }
    }

    var baseURLs: [URL] {
        lock.withLock { sentBaseURLs }
    }

    func replying(_ reply: @escaping Reply) {
        lock.withLock { self.reply = reply }
    }

    func send(
        _ request: HTTPRequest,
        body _: HTTPBody?,
        baseURL: URL,
        operationID _: String
    ) async throws -> (HTTPResponse, HTTPBody?) {
        let reply = lock.withLock {
            sentRequests.append(request)
            sentBaseURLs.append(baseURL)
            return self.reply
        }
        return try await reply(request, baseURL)
    }

    static func json(_ status: Int, _ json: String?) -> Reply {
        { _, _ in
            var fields = HTTPFields()
            fields[.contentType] = "application/json"
            return (
                HTTPResponse(status: .init(code: status), headerFields: fields),
                json.map { HTTPBody($0) }
            )
        }
    }
}

private actor Counter {
    private(set) var value = 0
    func increment() {
        value += 1
    }
}

private let tradesPage = #"{"items":[],"page":0,"page_size":20,"total":0}"#
private let someURL = URL(string: "https://one.example/api/v1")!

private func authenticated(
    _ transport: StubTransport,
    unauthorized: Counter = Counter(),
    latch: UnauthorizedLatch = UnauthorizedLatch(),
    baseURL: @escaping @Sendable () -> URL = { someURL }
) -> any APIProtocol {
    APIClientFactory.authenticated(
        onUnauthorized: { await unauthorized.increment() },
        latch: latch,
        transport: transport,
        token: { "token" },
        baseURL: baseURL
    )
}

private func listTrades(_ api: any APIProtocol) async throws {
    _ = try await api.list_trades(query: .init()).ok
}

private func failure(of call: () async throws -> Void) async -> APIError? {
    do {
        try await call()
        return nil
    } catch {
        return APIError.read(error)
    }
}

struct LiveAPITests {
    @Test func readsTheSuccessThroughTheGeneratedAccessor() async throws {
        let transport = StubTransport(StubTransport.json(200, tradesPage))
        try await listTrades(authenticated(transport))
        #expect(transport.requests.count == 1)
    }

    @Test func raisesTheCodeAndTheTranslatedMessageOfAnErrorBody() async {
        let transport = StubTransport(StubTransport.json(409, #"{"code":"card_already_reserved","error":"reserved"}"#))
        let error = await failure { try await listTrades(authenticated(transport)) }
        #expect(error?.code == "card_already_reserved")
        #expect(error?.status == 409)
        #expect(error?.message == "Cette carte est déjà réservée dans un autre échange.")
    }

    @Test func raisesTheGenericMessageForAnErrorWithoutBody() async {
        let transport = StubTransport(StubTransport.json(502, nil))
        let error = await failure { try await listTrades(authenticated(transport)) }
        #expect(error?.code == nil)
        #expect(error?.status == 502)
        #expect(error?.message == "Une erreur est survenue.")
    }

    @Test func raisesANetworkFailureWhenTheServerIsUnreachable() async {
        let transport = StubTransport { _, _ in throw URLError(.cannotConnectToHost) }
        let error = await failure { try await listTrades(authenticated(transport)) }
        #expect(error?.isNetworkFailure == true)
        #expect(error?.status == nil)
    }

    @Test func signsTheRequestWithTheToken() async throws {
        let transport = StubTransport(StubTransport.json(200, tradesPage))
        try await listTrades(authenticated(transport))
        #expect(transport.requests.first?.headerFields[.authorization] == "Bearer token")
    }

    @Test func callsOnUnauthorizedAndRaisesTheSessionExpiredMessage() async {
        let unauthorized = Counter()
        let transport = StubTransport(StubTransport.json(401, nil))
        let error = await failure { try await listTrades(authenticated(transport, unauthorized: unauthorized)) }
        #expect(await unauthorized.value == 1)
        #expect(error?.code == "unauthorized")
        #expect(error?.message == "Ta session a expiré, reconnecte-toi.")
    }

    @Test func signsOutOnceWhenSeveralRequestsFailAtTheSameTime() async {
        let unauthorized = Counter()
        let transport = StubTransport(StubTransport.json(401, nil))
        let api = authenticated(transport, unauthorized: unauthorized)
        await withTaskGroup(of: Void.self) { group in
            for _ in 0 ..< 8 {
                group.addTask { _ = await failure { try await listTrades(api) } }
            }
        }
        #expect(await unauthorized.value == 1)
        #expect(transport.requests.count == 8)
    }

    @Test func aSuccessBetweenTwoWavesOf401DoesNotSignOutTwice() async throws {
        let unauthorized = Counter()
        let transport = StubTransport(StubTransport.json(401, nil))
        let api = authenticated(transport, unauthorized: unauthorized)
        _ = await failure { try await listTrades(api) }
        transport.replying(StubTransport.json(200, tradesPage))
        try await listTrades(api)
        transport.replying(StubTransport.json(401, nil))
        _ = await failure { try await listTrades(api) }
        #expect(await unauthorized.value == 1)
    }

    @Test func signsOutAgainOnceTheLatchIsRearmedBySigningIn() async {
        let unauthorized = Counter()
        let latch = UnauthorizedLatch()
        let transport = StubTransport(StubTransport.json(401, nil))
        let api = authenticated(transport, unauthorized: unauthorized, latch: latch)
        _ = await failure { try await listTrades(api) }
        await latch.rearm()
        _ = await failure { try await listTrades(api) }
        #expect(await unauthorized.value == 2)
    }

    @Test func anAnonymousClientNeverSignsOutNorSendsAToken() async {
        let transport = StubTransport(StubTransport.json(401, nil))
        let api = APIClientFactory.anonymous(transport: transport, baseURL: { someURL })
        let error = await failure { try await listTrades(api) }
        #expect(error?.code == "unauthorized")
        #expect(transport.requests.first?.headerFields[.authorization] == nil)
    }

    @Test func usesTheCurrentBaseURLOnEveryRequest() async throws {
        let transport = StubTransport(StubTransport.json(200, tradesPage))
        let current = URLBox(someURL)
        let api = authenticated(transport, baseURL: { current.url })
        try await listTrades(api)
        current.url = URL(string: "https://two.example/api/v1")!
        try await listTrades(api)
        #expect(transport.baseURLs.map(\.host) == ["one.example", "two.example"])
    }
}

private final class URLBox: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: URL
    init(_ url: URL) {
        stored = url
    }

    var url: URL {
        get { lock.withLock { stored } }
        set { lock.withLock { stored = newValue } }
    }
}
