import Foundation
import GeneratorSupport
let text = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
let value = Int32(text.trimmingCharacters(in: .whitespacesAndNewlines))! + adjustment() + Int32(CommandLine.arguments.dropFirst(3).first ?? "0")!
try "func generatedValue() -> Int32 { \(value) }\n".write(toFile: CommandLine.arguments[2], atomically: true, encoding: .utf8)
