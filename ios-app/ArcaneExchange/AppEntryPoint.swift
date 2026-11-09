import SwiftUI

/// Launches the app, or an empty one when the process only hosts the unit tests.
///
/// The tests run inside the app (`@testable import`), so xcodebuild launches it first. They
/// never use its runtime: they build their own models on `FakeAPI`. Skipping Clerk, the image
/// pipeline and the API client makes that launch lighter, and keeps it from failing on what
/// the tests do not need, such as a missing Clerk publishable key.
@main
enum AppEntryPoint {
    @MainActor
    static func main() {
        if ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil {
            TestHostApp.main()
        } else {
            ArcaneExchangeApp.main()
        }
    }
}

private struct TestHostApp: App {
    var body: some Scene {
        WindowGroup {
            EmptyView()
        }
    }
}
