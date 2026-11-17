// swift-tools-version: 6.3
import PackageDescription

let package = Package(
    name: "SystemBridge",
    platforms: [.macOS(.v14)],
    products: [.library(name: "SystemBridge", type: .static, targets: ["SystemBridge"])],
    targets: [.target(name: "SystemBridge")]
)
