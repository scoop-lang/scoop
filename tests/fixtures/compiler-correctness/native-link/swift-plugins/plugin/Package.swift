// swift-tools-version: 6.3
import PackageDescription
let package = Package(name: "PluginKit",
    products: [.plugin(name: "Generate", targets: ["Generate"])],
    dependencies: [.package(path: "../support")],
    targets: [.plugin(name: "Generate", capability: .buildTool(), dependencies: ["Generator"]),
        .executableTarget(name: "Generator", dependencies: [.product(name: "GeneratorSupport", package: "support")], linkerSettings: [.unsafeFlags(["-Xlinker", "-headerpad_max_install_names"])])])
