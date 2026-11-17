// swift-tools-version: 6.3
import PackageDescription
let package = Package(name: "PluginBridge", platforms: [.macOS(.v14)],
    products: [.library(name: "PluginBridge", type: .static, targets: ["PluginBridge"])],
    dependencies: [.package(path: "../../plugin")],
    targets: [.target(name: "PluginBridge", exclude: ["value.txt"], plugins: [.plugin(name: "Generate", package: "plugin")])])
