// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AppleShell",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
        .visionOS(.v1),
    ],
    products: [
        .library(name: "AppleShellCore", targets: ["AppleShellCore"]),
        .executable(name: "SyncClipMac", targets: ["SyncClipMac"]),
    ],
    targets: [
        .systemLibrary(
            name: "clip_ffiFFI",
            path: "Generated"
        ),
        .target(
            name: "AppleShellCore",
            dependencies: ["clip_ffiFFI"],
            path: "Sources/AppleShellCore",
            linkerSettings: [
                .linkedLibrary("resolv"),
                .linkedFramework("Security"),
                .linkedLibrary("clip_ffi", .when(platforms: [.macOS])),
                .unsafeFlags(["-L", "lib"], .when(platforms: [.macOS])),
                .linkedFramework("AppKit", .when(platforms: [.macOS])),
                .linkedFramework("ServiceManagement", .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "SyncClipMac",
            dependencies: ["AppleShellCore"],
            path: "Sources/SyncClipMac"
        ),
        .testTarget(
            name: "AppleShellCoreTests",
            dependencies: ["AppleShellCore"],
            path: "Tests/AppleShellCoreTests",
            linkerSettings: [
                .linkedLibrary("clip_ffi", .when(platforms: [.macOS])),
                .unsafeFlags(["-L", "lib"], .when(platforms: [.macOS])),
            ]
        ),
    ]
)
