import SwiftUI

extension View {
    /// `refreshable` whose reload runs to its end. SwiftUI cancels the refresh task when the list
    /// re-renders mid-load, which would drop the reload half-way: it runs in its own task instead,
    /// and the spinner still waits for it.
    func refreshableToCompletion(_ action: @escaping @Sendable () async -> Void) -> some View {
        refreshable { await Task { await action() }.value }
    }
}
