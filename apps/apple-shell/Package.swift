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
                // clip_ffi: macOS SPM tests link lib/libclip_ffi.a; Xcode app links ClipFfi.xcframework.
                .linkedLibrary("clip_ffi", .when(platforms: [.macOS])),
                .unsafeFlags(["-L", "lib"], .when(platforms: [.macOS])),
                .linkedFramework("AppKit", .when(platforms: [.macOS])),
            ]
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
