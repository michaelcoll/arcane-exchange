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
    /// Teaches a `NavigationStack` about the trade screens.
    ///
    /// Goes *inside* the stack's content, like any other `navigationDestination`: a
    /// registration made outside the stack is never found, and pushing the route then lands
    /// on SwiftUI's blank "no destination" screen.
    func tradeDestinations() -> some View {
        navigationDestination(for: TradeDetailRoute.self) { TradeDetailView(route: $0) }
            .navigationDestination(for: TradeStepsRoute.self) { TradeStepsView(route: $0) }
    }

    /// Lets anything under this stack push a trade through `\.tradeNavigator`.
    ///
    /// Goes *on* the `NavigationStack`, not inside its content — the mirror image of
    /// `tradeDestinations()`: an environment value set inside the content never reaches the
    /// views `navigationDestination(for:)` builds, only one set above the stack does.
    func tradeNavigation(path: Binding<NavigationPath>) -> some View {
        environment(\.tradeNavigator) { path.wrappedValue.append($0) }
    }
}
