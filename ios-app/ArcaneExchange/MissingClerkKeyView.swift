import SwiftUI

/// Shown instead of the app by a build without a Clerk publishable key: without it nobody can
/// sign in, and Clerk traps as soon as it is read. Only a developer build ever shows it.
struct MissingClerkKeyView: View {
    var body: some View {
        ContentUnavailableView {
            Label("Clé Clerk manquante", systemImage: "key.slash")
        } description: {
            Text(
                "Copiez Config/Local.xcconfig.example vers Config/Local.xcconfig, "
                    + "renseignez CLERK_PUBLISHABLE_KEY puis relancez le build."
            )
        }
    }
}

#Preview {
    MissingClerkKeyView()
}
