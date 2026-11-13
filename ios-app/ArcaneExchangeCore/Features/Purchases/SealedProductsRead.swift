import APIClient

/// The sealed products read from the purchases, and why the read stopped when it did not reach
/// the end.
package struct SealedProductsRead {
    /// The sealed products read, most recent first. Possibly partial when `error` is set.
    package let products: [Purchase]
    /// French message, ready to show.
    package let error: String?
}

package extension APIProtocol {
    /// Reads every page of the purchases, the sealed products being among them whatever their age.
    /// A failed read leaves the products read so far: a caller can go on without.
    func readSealedProducts() async -> SealedProductsRead {
        var purchases: [Purchase] = []
        var page: Int32 = 0
        var message: String?
        do {
            while true {
                let query = Operations.list_purchases.Input.Query(page: page, page_size: 100)
                let response = try await list_purchases(query: query).ok.body.json
                purchases += response.items
                page += 1
                if response.items.isEmpty || purchases.count >= Int(response.total) {
                    break
                }
            }
        } catch {
            message = APIError.read(error).message
        }
        return SealedProductsRead(products: purchases.filter { $0.product_type != nil }, error: message)
    }
}
