import SwiftUI

extension EnvironmentValues {
    /// Pushes a trade onto the enclosing stack.
    ///
    /// The owners screen only learns the trade's id after `POST /trades` has answered, so it
    /// cannot use a `NavigationLink(value:)`; it needs to push after the fact. Handing it the
    /// stack's path through the environment keeps every tab's navigation state where it
    /// belongs — in the view that owns the `NavigationStack`.
    @Entry var tradeNavigator: (TradeDetailRoute) -> Void = { _ in }
}

extension View {
    /// Teaches a `NavigationStack` about the trade screens and lets anything inside it push one.
    func tradeDestinations(path: Binding<NavigationPath>) -> some View {
        navigationDestination(for: TradeDetailRoute.self) { TradeDetailView(route: $0) }
            .navigationDestination(for: TradeStepsRoute.self) { TradeStepsView(route: $0) }
            .environment(\.tradeNavigator) { path.wrappedValue.append($0) }
    }
}
