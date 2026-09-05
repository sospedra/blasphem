import PackagePlugin
import Foundation

@main
struct BlasphemAssets: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) throws -> [Command] {
        try commands(tool: context.tool(named: "BlasphemAssetGenerator").path,
                     root: context.package.directory, output: context.pluginWorkDirectory,
                     xcode: false)
    }

    func commands(tool: Path, root: Path, output: Path, xcode: Bool) throws -> [Command] {
        let configuration = root.appending("blasphem.json")
        let names = ["BlasphemAssets.bundle", "BlasphemConfiguration.generated.swift"]
        return [.buildCommand(displayName: "Configure Blasphem language assets", executable: tool,
            arguments: [root.appending("blasphem.json").string, output.appending("Generated").string, xcode ? "xcode" : "swiftpm"],
            inputFiles: [configuration], outputFiles: names.map { output.appending("Generated").appending($0) })]
    }
}

#if canImport(XcodeProjectPlugin)
import XcodeProjectPlugin
extension BlasphemAssets: XcodeBuildToolPlugin {
    func createBuildCommands(context: XcodePluginContext, target: XcodeTarget) throws -> [Command] {
        try commands(tool: context.tool(named: "BlasphemAssetGenerator").path,
                     root: context.xcodeProject.directory, output: context.pluginWorkDirectory,
                     xcode: true)
    }
}
#endif
