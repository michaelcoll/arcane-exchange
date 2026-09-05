import SwiftUI

/// Root navigation for a signed-in user.
///
/// Native `TabView` with the three destinations from the iOS mockup
/// (`maquette/Arcane Exchange iOS.html`): Collection, Échanges, and a dedicated
/// search tab (`role: .search`) that the system pins to the trailing edge of the
/// tab bar. Réglages is not a tab: like the mockup, it opens from the account
/// avatar in each screen's nav bar (`accountToolbar()`).
struct RootTabView: View {
    /// Named so it does not shadow SwiftUI's `Tab` view type used in `body`.
    enum Destination: Hashable {
        case collection
        case trades
        case search
    }

    @State private var selection: Destination = .collection
    @State private var searchText = ""

    var body: some View {
        TabView(selection: $selection) {
            Tab("Collection", systemImage: "rectangle.stack", value: .collection) {
                CollectionView()
            }

            Tab("Échanges", systemImage: "arrow.left.arrow.right", value: .trades) {
                TradesView()
            }

            Tab("Rechercher", systemImage: "magnifyingglass", value: .search, role: .search) {
                SearchView(searchText: $searchText)
            }
        }
    }
}

#Preview {
    RootTabView()
}
