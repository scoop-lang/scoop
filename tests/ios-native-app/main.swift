import Dispatch
import Foundation
import ScoopProbe

struct App: @unchecked Sendable {
    let api: ScoopTestAppApi

    func run(_ input: Int32) {
        precondition(api.run!(input, api.context) == input * 5 + 35)
    }

    func close() {
        api.close!(api.close_context)
    }
}

var host = ScoopTestHostApi(service: { $0 * 5 }, fail: 0)
var api = ScoopTestAppApi()
var error = ScoopAppError()
if CommandLine.arguments.contains("--fail") {
    host.fail = 1
    precondition(scoop_app_init(&host, &api, &error) == SCOOP_APP_INIT_FAILED)
    let message = String(decoding: UnsafeBufferPointer(start: error.bytes, count: Int(error.length)), as: UTF8.self)
    precondition(message.contains("IllegalStateException"))
    print("scoop-swift-init-failure-ok")
} else {
    precondition(scoop_app_init(&host, &api, &error) == SCOOP_APP_INIT_OK)
    let app = App(api: api)
    app.run(11)
    let finished = DispatchSemaphore(value: 0)
    DispatchQueue.global().async {
        precondition(!Thread.isMainThread)
        app.run(13)
        finished.signal()
    }
    precondition(finished.wait(timeout: .now() + 10) == .success)
    app.close()
    print("scoop-swift-moving-gc-ok")
}
