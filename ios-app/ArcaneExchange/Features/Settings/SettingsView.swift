import SwiftUI

/// Réglages tab — profile, tradeability settings, import, app preferences.
/// Scaffold only; content to be built against `GET /me` and related endpoints.
struct SettingsView: View {
    var body: some View {
        NavigationStack {
            ContentUnavailableView(
                "Réglages à venir",
                systemImage: "slider.horizontal.3",
                description: Text("Profil, échangeabilité de la collection et préférences de l'app.")
            )
            .navigationTitle("Réglages")
        }
    }
}

#Preview {
    SettingsView()
}
