import Observation

/// App-wide signal raised each time an import may have changed the collection: when it finishes
/// successfully, or when its sheet closes while it is still tracked. The import sheet lives in
/// Settings while the collection grid lives in its own tab: the grid watches `collectionChanges`
/// to reload, whichever tab the Settings were opened from.
@MainActor
@Observable
package final class CollectionImportSignal {
    /// Changes on every raise; only the change matters, not the value.
    package private(set) var collectionChanges = 0

    package init() {}

    package func collectionChanged() {
        collectionChanges += 1
    }
}
