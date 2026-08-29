import SwiftUI

/// Root navigation for a signed-in user.
///
/// Native `TabView` with the four destinations from the iOS mockup
/// (`maquette/Arcane Exchange iOS.html`): Collection, Échanges, Réglages, and a
/// dedicated search tab (`role: .search`) that the system pins to the trailing
/// edge of the tab bar. Notifications live in the nav-bar bell, not a tab.
struct RootTabView: View {
    /// Named so it does not shadow SwiftUI's `Tab` view type used in `body`.
    enum Destination: Hashable {
        case collection
        case trades
        case settings
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

            Tab("Réglages", systemImage: "slider.horizontal.3", value: .settings) {
                SettingsView()
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
