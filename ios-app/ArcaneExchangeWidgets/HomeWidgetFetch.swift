import APIClient
import ClerkKit
import Foundation

/// The Clerk session the app opened, read from the shared Keychain.
@MainActor
enum WidgetSession {
    private static var isConfigured = false

    /// Configures Clerk on first use — it restores the cached session synchronously — and
    /// reports whether there is one. The token itself is refreshed by `ClerkAuthMiddleware`
    /// on each request, as in the app.
    static func isSignedIn() -> Bool {
        if !isConfigured {
            ClerkSetup.configure()
            isConfigured = true
        }
        return Clerk.shared.session != nil
    }
}

/// The widgets' API calls: the same endpoints as the web home page's two tiles.
enum HomeWidgetFetch {
    /// The trade statuses the web tile lists — the active trades.
    private static let activeStatuses: [TradeStatusParam] = [.PENDING, .ONE_ACCEPTED, .FULLY_ACCEPTED]

    static func collection() async throws -> CollectionSnapshot {
        let client = APIClientProvider.shared
        // No dates: the endpoint defaults to the last 30 days.
        async let history = client.get_collection_price_history(query: .init())
        async let stats = client.get_collection_stats()

        let entries: [PriceHistoryEntry] = switch try await history {
        case let .ok(response): try response.body.json
        case .badRequest: throw APIClientError.undocumented(statusCode: 400)
        case .unauthorized: throw APIClientError.unauthorized
        case let .undocumented(statusCode, _): throw APIClientError.undocumented(statusCode: statusCode)
        }
        let counts = switch try await stats {
        case let .ok(response): try response.body.json
        case .unauthorized: throw APIClientError.unauthorized
        case let .undocumented(statusCode, _): throw APIClientError.undocumented(statusCode: statusCode)
        }
        return CollectionSnapshot(
            history: entries,
            totalCards: Int(counts.total_cards),
            uniqueCards: Int(counts.unique_cards)
        )
    }

    static func trades() async throws -> TradesSnapshot {
        let query = Operations.list_trades.Input.Query(
            page: 0,
            page_size: Int32(TradesSnapshot.rowCount),
            status: activeStatuses
        )
        let page = switch try await APIClientProvider.shared.list_trades(query: query) {
        case let .ok(response): try response.body.json
        case .badRequest: throw APIClientError.undocumented(statusCode: 400)
        case .unauthorized: throw APIClientError.unauthorized
        case let .undocumented(statusCode, _): throw APIClientError.undocumented(statusCode: statusCode)
        }
        return TradesSnapshot(
            total: Int(page.total),
            trades: page.items.map { trade in
                ActiveTrade(
                    id: trade.id,
                    partnerUsername: trade.partner_username,
                    myCardCount: Int(trade.my_card_count),
                    partnerCardCount: Int(trade.partner_card_count),
                    status: trade.status
                )
            }
        )
    }
}
