import XCTest

final class InteractionTests: XCTestCase {
    func testWindowTouches() {
        let app = XCUIApplication(bundleIdentifier: "dev.scoop.ios-probe.window")
        app.launchArguments = ["--touch"]
        app.launch()
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.4))
        let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.6))
        let result = app.descendants(matching: .any).matching(identifier: "scoop-touch-result").firstMatch
        XCTAssertTrue(result.waitForExistence(timeout: 5))
        start.tap()
        XCTAssertEqual(result.value as? String, "tap delivered")
        start.press(forDuration: 0.15, thenDragTo: end)
        XCTAssertEqual(result.value as? String, "drag delivered")
        result.twoFingerTap()
        XCTAssertEqual(result.value as? String, "two contacts delivered")
        start.press(forDuration: 0.5)
        XCTAssertEqual(result.value as? String, "contact cancelled")
    }
}
