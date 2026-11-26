import XCTest

final class InteractionTests: XCTestCase {
    func testHardwareKeys() {
        let app = XCUIApplication(bundleIdentifier: "dev.scoop.ios-probe.window")
        app.launchArguments = ["--keys"]
        app.launch()
        let result = app.descendants(matching: .any).matching(identifier: "scoop-key-result").firstMatch
        XCTAssertTrue(result.waitForExistence(timeout: 5))
        app.typeKey("k", modifierFlags: .command)
        XCTAssertEqual(result.value as? String, "Command-K delivered")
        app.typeKey(.downArrow, modifierFlags: [])
        XCTAssertEqual(result.value as? String, "arrow delivered")
    }

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
