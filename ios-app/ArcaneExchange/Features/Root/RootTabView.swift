import APIClient
import ArcaneExchangeCore
import SwiftUI

/// Root navigation for a signed-in user.
///
/// Réglages is not a tab: it opens from the account avatar in each screen's nav bar
/// (`accountToolbar()`). The search tab (`role: .search`) sits at the trailing edge: set apart
/// up to iOS 26, inside the bar from iOS 27.
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
                ScannerView(index: scanIndex, api: api)
            }

            Tab("Échanges", systemImage: "arrow.left.arrow.right", value: .trades) {
                TradesView(api: api)
            }

            Tab("Rechercher", systemImage: "magnifyingglass", value: .search, role: .search) {
                SearchView(searchText: $searchText, api: api)
            }
        }
        // Tapping the magnifying glass turns the tab bar into the search field and opens
        // the keyboard straight away.
        .tabViewSearchActivation(.searchTabSelection)
        // At launch, so the index is there by the time the player opens the Scanner tab.
        .task { await scanIndex.refresh() }
    }
}

#Preview {
    RootTabView()
        .environment(AppRouter())
        .environment(CollectionImportSignal())
}
