import Foundation

@c(scoop_swift_resource)
public func resourceValue() -> Int32 {
    let bundle = Bundle.module
    let directory = bundle.bundleURL.deletingLastPathComponent().resolvingSymlinksInPath()
    guard directory == Bundle.main.resourceURL?.resolvingSymlinksInPath() else {
        return -1
    }
    guard let url = bundle.url(forResource: "value", withExtension: "txt"),
          let text = try? String(contentsOf: url, encoding: .utf8),
          let value = Int32(text.trimmingCharacters(in: .whitespacesAndNewlines)) else {
        return -2
    }
    return value
}
