import APIClient
import Foundation
import HTTPTypes
import OpenAPIRuntime
import Testing

@testable import ArcaneExchangeCore

/// What validating a scan batch asks of the networking layer: the widget reload and the cards the
/// catalog refused, carried by the error.
struct ScanBatchNetworkingTests {
    @Test func validatingAScanBatchReloadsTheCollectionWidget() {
        #expect(WidgetsReloadMiddleware.table[Operations.validate_scan_batch.id] == [.collection])
    }

    @Test func anErrorCarriesTheCardsTheCatalogDoesNotHave() async throws {
        let json = """
        {"error":"unknown cards","code":"card_not_in_catalog",
         "lines":[{"set_code":"MOM","collector_number":"999","language_code":"FR"}]}
        """
        let middleware = APIErrorMiddleware(onUnauthorized: nil, latch: UnauthorizedLatch())

        let error = await #expect(throws: APIError.self) {
            _ = try await middleware.intercept(
                HTTPRequest(method: .post, scheme: nil, authority: nil, path: "/collection/scan-batches"),
                body: nil,
                baseURL: URL(string: "https://example.test")!,
                operationID: Operations.validate_scan_batch.id
            ) { _, _, _ in
                (HTTPResponse(status: .notFound), HTTPBody(json))
            }
        }

        #expect(error?.code == "card_not_in_catalog")
        #expect(error?.unknownCards == [.init(collector_number: "999", language_code: "FR", set_code: "MOM")])
    }

    @Test func anErrorWithoutLinesCarriesNoCard() {
        #expect(APIError(code: "binder_not_found", status: 404).unknownCards.isEmpty)
    }
}
