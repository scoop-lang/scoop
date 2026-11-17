// swift-tools-version: 6.3
import PackageDescription
let package = Package(
    name: "NativeVendor",
    products: [.library(name: "NativeKit", targets: ["NativeKit"])],
    targets: [.binaryTarget(name: "NativeKit", path: "NativeKit.xcframework")]
)
