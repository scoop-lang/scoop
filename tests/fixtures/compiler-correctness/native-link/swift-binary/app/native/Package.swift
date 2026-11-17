// swift-tools-version: 6.3
import PackageDescription
let package = Package(
    name: "BinaryBridge",
    platforms: [.macOS(.v14)],
    products: [.library(name: "BinaryBridge", type: .static, targets: ["BinaryBridge"])],
    dependencies: [.package(path: "../../vendor")],
    targets: [.target(name: "BinaryBridge", dependencies: [.product(name: "NativeKit", package: "vendor")])]
)
