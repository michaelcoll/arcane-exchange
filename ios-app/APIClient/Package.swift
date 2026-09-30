// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "APIClient",
    platforms: [.iOS(.v18), .macOS(.v13)],
    products: [
        .library(name: "APIClient", targets: ["APIClient"])
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-openapi-generator", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-openapi-runtime", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-openapi-urlsession", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "APIClient",
            dependencies: [
                .product(name: "OpenAPIRuntime", package: "swift-openapi-runtime"),
                .product(name: "OpenAPIURLSession", package: "swift-openapi-urlsession"),
            ],
            // Only generated code lives here, and the generator writes `public import` for every
            // module whether a file exposes it or not: one UnusedImportAccess warning per file.
            swiftSettings: [.unsafeFlags(["-suppress-warnings"])]
        )
    ]
)
