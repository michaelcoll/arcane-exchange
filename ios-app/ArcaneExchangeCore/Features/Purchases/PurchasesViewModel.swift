import APIClient
import Foundation

/// Backs the Achats screen: the player's purchases, most recent first, and the deletion of one
/// (`/purchases`). Creating and editing a sealed product is `SealedProductFormViewModel`.
@MainActor
@Observable
package final class PurchasesViewModel {
    private let api: any APIProtocol

    /// The loaded purchases and their loading state.
    package let paginator: Paginator<Purchase>

    /// Set when a deletion failed: the list is intact. French message, ready to show.
    package var writeError: String?
    /// Whether a deletion is waiting on the server: the screen holds still meanwhile.
    package private(set) var isWriting = false

    package init(api: any APIProtocol) {
        self.api = api
        paginator = Paginator(fetch: { page, pageSize in
            let query = Operations.list_purchases.Input.Query(page: page, page_size: pageSize)
            let response = try await api.list_purchases(query: query).ok.body.json
            return PaginatorPage(items: response.items, total: Int(response.total))
        })
    }

    /// Reloads from page 0: the first load, pull-to-refresh, and after a creation or an edit.
    package func reload() async {
        await paginator.reload()
    }

    /// Deletes the purchase, never a card of the collection, then reloads the list.
    package func delete(_ purchase: Purchase) async {
        isWriting = true
        defer { isWriting = false }
        do {
            _ = try await api.delete_purchase(path: .init(id: purchase.id)).noContent
        } catch {
            writeError = APIError.read(error).message
            return
        }
        await paginator.reload()
    }
}
