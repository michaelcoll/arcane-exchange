import SwiftUI

extension View {
    /// Surfaces a rejected write from `AccountSettingsViewModel`.
    ///
    /// Every drawer that writes carries it: SwiftUI cannot present an alert on a view that a
    /// sheet already covers, so putting it only on the Réglages screen would silently swallow
    /// every failure raised from a sub-drawer.
    func writeErrorAlert(_ model: AccountSettingsViewModel) -> some View {
        alert(
            "Modification impossible",
            isPresented: Binding(
                get: { model.writeError != nil },
                set: { isPresented in
                    if !isPresented {
                        model.writeError = nil
                    }
                }
            ),
            presenting: model.writeError,
            actions: { _ in Button("OK", role: .cancel) {} },
            message: { Text($0.message) }
        )
    }
}
