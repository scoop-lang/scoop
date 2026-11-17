// swift-tools-version: 6.3
import PackageDescription

let package = Package(
    name: "ResourceBridge",
    platforms: [.macOS(.v14)],
    products: [.library(name: "ResourceBridge", type: .static, targets: ["ResourceBridge"])],
    targets: [.target(name: "ResourceBridge", resources: [.process("Resources")])]
)
