import APIClient
import ArcaneExchangeCore
import SwiftUI

/// Root navigation for a signed-in user.
///
/// Native `TabView` with the destinations from the iOS mockup
/// (`maquette/Arcane Exchange iOS.html`): Collection, Scanner, Échanges, and a dedicated
/// search tab (`role: .search`) that the system pins to the trailing edge of the
/// tab bar. Réglages is not a tab: like the mockup, it opens from the account
/// avatar in each screen's nav bar (`accountToolbar()`).
struct RootTabView: View {
    /// Holds the selected tab, so a widget tap can switch it.
    @Environment(AppRouter.self) private var router
    @Environment(\.api) private var api
    @Environment(\.scanIndex) private var scanIndex
    @State private var searchText = ""

    var body: some View {
        @Bindable var router = router
        TabView(selection: $router.tab) {
            Tab("Collection", systemImage: "rectangle.stack", value: .collection) {
                CollectionView(api: api)
            }

            Tab("Scanner", systemImage: "viewfinder", value: .scanner) {
                ScannerView(index: scanIndex)
            }

            Tab("Échanges", systemImage: "arrow.left.arrow.right", value: .trades) {
                TradesView(api: api)
            }

            Tab("Rechercher", systemImage: "magnifyingglass", value: .search, role: .search) {
                SearchView(searchText: $searchText, api: api)
            }
        }
        // At launch, so the index is there by the time the player opens the Scanner tab.
        .task { await scanIndex.refresh() }
    }
}

#Preview {
    RootTabView()
        .environment(AppRouter())
}
