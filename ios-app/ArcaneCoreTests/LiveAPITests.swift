import APIClient
import Foundation
import HTTPTypes
import OpenAPIRuntime
import Testing

@testable import ArcaneCore

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

private final class WidgetRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var recorded: [HomeWidgetKind] = []

    var kinds: [HomeWidgetKind] {
        lock.withLock { recorded }
    }

    func record(_ kind: HomeWidgetKind) {
        lock.withLock { recorded.append(kind) }
    }
}

/// One call per operation the widgets table is checked against.
private enum MutationCall: String, CaseIterable, CustomTestStringConvertible {
    case createTrade, addTradeCard, removeTradeCard, acceptTrade, confirmTrade, abandonTrade
    case rateTrade, importCards, setTradeVisibility, setTradeRarityFilter, addTradeBinder, removeTradeBinder
    case createBinder, renameBinder, deleteBinder, addCollectionEntry, updateCollectionEntry, moveCollectionEntry
    case removeCollectionEntry

    static let reloadingTrades: [MutationCall] = [
        .createTrade, .addTradeCard, .removeTradeCard, .acceptTrade, .confirmTrade, .abandonTrade
    ]
    static let reloadingCollection: [MutationCall] = [
        .createBinder, .renameBinder, .deleteBinder, .addCollectionEntry, .updateCollectionEntry,
        .moveCollectionEntry, .removeCollectionEntry
    ]
    static let reloadingNothing: [MutationCall] = [
        .rateTrade, .importCards, .setTradeVisibility, .setTradeRarityFilter, .addTradeBinder, .removeTradeBinder
    ]

    var testDescription: String {
        rawValue
    }

    /// The status of a successful answer.
    var successStatus: Int {
        switch self {
        case .createTrade, .createBinder, .addCollectionEntry: 201
        case .importCards: 202
        case .renameBinder, .updateCollectionEntry, .moveCollectionEntry: 200
        default: 204
        }
    }

    var successBody: String? {
        switch self {
        case .createTrade: #"{"id":"t-1"}"#
        case .importCards: #"{"id":"imp-1"}"#
        case .createBinder, .renameBinder: #"{"copies":0,"id":"b-1","is_open":false,"name":"Bulk"}"#
        case .addCollectionEntry, .updateCollectionEntry, .moveCollectionEntry: #"{"entries":[],"reserved_quantity":{"foil":0,"normal":0}}"#
        default: nil
        }
    }

    func call(_ api: any APIProtocol) async throws {
        switch self {
        case .createTrade:
            _ = try await api.create_trade(body: .json(.init(respondent_username: "bob"))).created.body.json
        case .addTradeCard:
            _ = try await api.add_trade_card(
                path: .init(trade_id: "t-1"),
                body: .json(.init(
                    collector_number: "1", foil: false, language_code: "EN", owner_username: "bob", set_code: "lea"
                ))
            ).noContent
        case .removeTradeCard:
            _ = try await api.remove_trade_card(
                path: .init(trade_id: "t-1"),
                body: .json(.init(
                    collector_number: "1", foil: false, language_code: "EN", owner_username: "bob", set_code: "lea"
                ))
            ).noContent
        case .acceptTrade:
            _ = try await api.accept_trade(path: .init(trade_id: "t-1")).noContent
        case .confirmTrade:
            _ = try await api.confirm_trade(path: .init(trade_id: "t-1")).noContent
        case .abandonTrade:
            _ = try await api.abandon_trade(path: .init(trade_id: "t-1")).noContent
        case .rateTrade:
            _ = try await api.rate_trade(path: .init(trade_id: "t-1"), body: .json(.init(rating: 5))).noContent
        case .importCards:
            _ = try await api.import_cards(.init(body: .plainText("csv"))).accepted.body.json
        case .setTradeVisibility:
            _ = try await api.set_trade_visibility(body: .json(.init(visibility: ._private))).noContent
        case .setTradeRarityFilter:
            _ = try await api.set_trade_rarity_filter(
                body: .json(.init(is_open: true, kept_copies: 0, rarity: "rare"))
            ).noContent
        case .addTradeBinder:
            _ = try await api.add_trade_binder(body: .json(.init(binder_name: "b"))).noContent
        case .removeTradeBinder:
            _ = try await api.remove_trade_binder(path: .init(name: "b")).noContent
        case .createBinder:
            _ = try await api.create_binder(body: .json(.init(name: "Bulk"))).created
        case .renameBinder:
            _ = try await api.rename_binder(path: .init(id: "b-1"), body: .json(.init(name: "Bulk"))).ok
        case .deleteBinder:
            _ = try await api.delete_binder(path: .init(id: "b-1")).noContent
        case .addCollectionEntry:
            _ = try await api.add_collection_entry(body: .json(.init(
                collector_number: "1", foil: false, language_code: "FR", quantity: 1, set_code: "lea"
            ))).created
        case .updateCollectionEntry:
            _ = try await api.update_collection_entry(path: .init(id: "e-1"), body: .json(.init(quantity: 2))).ok
        case .moveCollectionEntry:
            _ = try await api.move_collection_entry(path: .init(id: "e-1"), body: .json(.init(quantity: 1))).ok
        case .removeCollectionEntry:
            _ = try await api.remove_collection_entry(path: .init(id: "e-1")).noContent
        }
    }
}

private let tradesPage = #"{"items":[],"page":0,"page_size":20,"total":0}"#
private let someURL = URL(string: "https://one.example/api/v1")!

private func authenticated(
    _ transport: StubTransport,
    unauthorized: Counter = Counter(),
    latch: UnauthorizedLatch = UnauthorizedLatch(),
    widgets: WidgetRecorder = WidgetRecorder(),
    baseURL: @escaping @Sendable () -> URL = { someURL }
) -> any APIProtocol {
    APIClientFactory.authenticated(
        onUnauthorized: { await unauthorized.increment() },
        onWidgetsChanged: { widgets.record($0) },
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

    @Test(arguments: MutationCall.reloadingTrades)
    fileprivate func reloadsTheTradesWidgetOnceOnASuccessOf(_ operation: MutationCall) async throws {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(operation.successStatus, operation.successBody))
        try await operation.call(authenticated(transport, widgets: widgets))
        #expect(widgets.kinds == [.trades])
    }

    @Test(arguments: MutationCall.reloadingCollection)
    fileprivate func reloadsTheCollectionWidgetOnceOnASuccessOf(_ operation: MutationCall) async throws {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(operation.successStatus, operation.successBody))
        try await operation.call(authenticated(transport, widgets: widgets))
        #expect(widgets.kinds == [.collection])
    }

    @Test(arguments: MutationCall.reloadingCollection)
    fileprivate func reloadsTheCollectionWidgetWhenTheServerRefuses(_ operation: MutationCall) async {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(409, #"{"code":"binder_not_empty","error":"not empty"}"#))
        let error = await failure { try await operation.call(authenticated(transport, widgets: widgets)) }
        #expect(widgets.kinds == [.collection])
        #expect(error?.code == "binder_not_empty")
    }

    @Test(arguments: MutationCall.reloadingTrades)
    fileprivate func reloadsTheTradesWidgetWhenTheServerRefuses(_ operation: MutationCall) async {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(409, #"{"code":"card_already_reserved","error":"reserved"}"#))
        let error = await failure { try await operation.call(authenticated(transport, widgets: widgets)) }
        #expect(widgets.kinds == [.trades])
        #expect(error?.code == "card_already_reserved")
        #expect(error?.message == "Cette carte est déjà réservée dans un autre échange.")
    }

    @Test(arguments: MutationCall.reloadingTrades)
    fileprivate func reloadsNothingWhenTheServerIsUnreachableOn(_ operation: MutationCall) async {
        let widgets = WidgetRecorder()
        let transport = StubTransport { _, _ in throw URLError(.cannotConnectToHost) }
        let error = await failure { try await operation.call(authenticated(transport, widgets: widgets)) }
        #expect(widgets.kinds.isEmpty)
        #expect(error?.isNetworkFailure == true)
    }

    @Test(arguments: MutationCall.reloadingNothing)
    fileprivate func reloadsNothingOnASuccessOf(_ operation: MutationCall) async throws {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(operation.successStatus, operation.successBody))
        try await operation.call(authenticated(transport, widgets: widgets))
        #expect(transport.requests.count == 1)
        #expect(widgets.kinds.isEmpty)
    }

    @Test(arguments: MutationCall.reloadingTrades)
    fileprivate func reloadsNothingWhenTheCallIsCancelledOn(_ operation: MutationCall) async {
        let widgets = WidgetRecorder()
        let transport = StubTransport { _, _ in throw CancellationError() }
        let error = await failure { try await operation.call(authenticated(transport, widgets: widgets)) }
        #expect(error != nil)
        #expect(widgets.kinds.isEmpty)
    }

    @Test func reloadsTwiceWhenTheTradeIsCreatedButItsCardIsRefused() async {
        let widgets = WidgetRecorder()
        let created = StubTransport.json(201, #"{"id":"t-1"}"#)
        let refused = StubTransport.json(409, #"{"code":"card_already_reserved","error":"reserved"}"#)
        let transport = StubTransport { request, baseURL in
            request.path?.hasSuffix("/cards") == true
                ? try await refused(request, baseURL)
                : try await created(request, baseURL)
        }
        let api = authenticated(transport, widgets: widgets)
        let error = await failure {
            _ = try await api.create_trade(body: .json(.init(respondent_username: "bob"))).created.body.json
            try await MutationCall.addTradeCard.call(api)
        }
        #expect(widgets.kinds == [.trades, .trades])
        #expect(error?.code == "card_already_reserved")
    }

    @Test func aReadReloadsNothing() async throws {
        let widgets = WidgetRecorder()
        let transport = StubTransport(StubTransport.json(200, tradesPage))
        try await listTrades(authenticated(transport, widgets: widgets))
        #expect(widgets.kinds.isEmpty)
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
