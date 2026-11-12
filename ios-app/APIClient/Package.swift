// swift-tools-version: 6.0
import PackageDescription

/// The `*Shared` targets have no code of their own: without it the linker loads none of their
/// dependencies' objects, and the app fails on undefined NukeUI symbols.
let loadEverything: [LinkerSetting] = [.unsafeFlags(["-Xlinker", "-all_load"])]

let package = Package(
    name: "APIClient",
    platforms: [.iOS(.v18), .macOS(.v13)],
    products: [
        .library(name: "APIClient", targets: ["APIClient"]),
        // Clerk and Nuke, forced dynamic so the app, the framework and the widget extension share
        // one copy of each. Linked as Xcode's automatic products, ClerkKitUI and NukeUI went
        // static into the app with their own copy of ClerkKit and Nuke, whose singletons
        // (`Clerk.shared`, `ImagePipeline.shared`) the framework never configured.
        // One product per library, UI included: a core product and a UI one depending on it
        // would put the core target in both. `scripts/check-app-linking.sh` guards it.
        .library(name: "ClerkKitShared", type: .dynamic, targets: ["ClerkKitShared"]),
        .library(name: "NukeShared", type: .dynamic, targets: ["NukeShared"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-openapi-generator", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-openapi-runtime", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-openapi-urlsession", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-http-types", from: "1.0.0"),
        .package(url: "https://github.com/clerk/clerk-ios", from: "1.2.0"),
        .package(url: "https://github.com/kean/Nuke", from: "13.2.0"),
    ],
    targets: [
        .target(
            name: "ClerkKitShared",
            dependencies: [
                .product(name: "ClerkKit", package: "clerk-ios"),
                .product(name: "ClerkKitUI", package: "clerk-ios"),
            ],
            linkerSettings: loadEverything
        ),
        .target(
            name: "NukeShared",
            dependencies: [
                .product(name: "Nuke", package: "Nuke"),
                .product(name: "NukeUI", package: "Nuke"),
            ],
            linkerSettings: loadEverything
        ),
        .target(
            name: "APIClient",
            dependencies: [
                .product(name: "OpenAPIRuntime", package: "swift-openapi-runtime"),
                .product(name: "OpenAPIURLSession", package: "swift-openapi-urlsession"),
                // The generated code names HTTPTypes directly: declared so the dynamic product
                // Xcode builds once several targets (ArcaneExchangeCore, the widgets) link APIClient
                // links it too, instead of failing on undefined symbols.
                .product(name: "HTTPTypes", package: "swift-http-types"),
            ],
            // Only generated code lives here, and the generator writes `public import` for every
            // module whether a file exposes it or not: one UnusedImportAccess warning per file.
            swiftSettings: [.unsafeFlags(["-suppress-warnings"])]
        )
    ]
)
