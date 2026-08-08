// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MacosShell",
    platforms: [
        .macOS(.v13),
    ],
    products: [
        .executable(name: "MacosShell", targets: ["MacosShell"]),
        .library(name: "MacosShellCore", targets: ["MacosShellCore"]),
    ],
    targets: [
        .systemLibrary(
            name: "clip_ffiFFI",
            path: "Generated"
        ),
        .target(
            name: "MacosShellCore",
            dependencies: ["clip_ffiFFI"],
            path: "Sources/MacosShellCore",
            linkerSettings: [
                .linkedLibrary("clip_ffi"),
                .linkedLibrary("resolv"),
                .linkedFramework("AppKit"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
                .unsafeFlags([
                    "-L",
                    "lib",
                ]),
            ]
        ),
        .executableTarget(
            name: "MacosShell",
            dependencies: ["MacosShellCore"],
            path: "Sources/MacosShell",
            exclude: ["Info.plist"]
        ),
        .testTarget(
            name: "MacosShellTests",
            dependencies: ["MacosShellCore"],
            path: "Tests/MacosShellTests"
        ),
    ]
)
