import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

struct SearchCardTests {
    /// The detail screens read a `CollectionCard`: a search result opens them with its public
    /// figures and no collection entry.
    @Test func opensTheDetailWithoutACollectionEntry() {
        let result = searchCard("abc").asCollectionCard

        #expect(result.scryfall_id == "abc")
        #expect(result.name == "Card abc")
        #expect(result.collection_entry == nil)
    }
}
