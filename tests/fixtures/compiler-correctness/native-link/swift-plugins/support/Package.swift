// swift-tools-version: 6.3
import PackageDescription
let package = Package(name: "GeneratorSupport",
    products: [.library(name: "GeneratorSupport", targets: ["GeneratorSupport"])],
    targets: [.target(name: "GeneratorSupport", linkerSettings: [.linkedLibrary("z")])])
