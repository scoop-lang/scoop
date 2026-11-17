import PackagePlugin
import Foundation
@main struct Generate: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) async throws -> [Command] {
        let tool = try context.tool(named: "Generator")
        let input = URL(fileURLWithPath: target.directory.string).appendingPathComponent("value.txt")
        let output = context.pluginWorkDirectoryURL.appendingPathComponent("Generated.swift")
        return [.buildCommand(displayName: "Generate value", executable: tool.url,
            arguments: [input.path, output.path, "10"], inputFiles: [input], outputFiles: [output])]
    }
}
