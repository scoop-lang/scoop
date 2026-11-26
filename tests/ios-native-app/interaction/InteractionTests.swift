import XCTest
import UIKit

final class InteractionTests: XCTestCase {
    func testSurfacePresentation() throws {
        let app = XCUIApplication(bundleIdentifier: "dev.scoop.ios-probe.surface")
        app.launchArguments = ["--observe"]
        app.launch()
        let surface = app.descendants(matching: .any).matching(identifier: "scoop-surface").firstMatch
        XCTAssertTrue(surface.waitForExistence(timeout: 8))
        XCTAssertEqual(surface.value as? String, "presented")
        let green = try centerPixel(of: app.screenshot())
        XCTAssertGreaterThan(Int(green[1]), Int(max(green[0], green[2])) + 50)
        XCUIDevice.shared.press(.home)
        app.activate()
        let resumed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == 'resumed'"), object: surface)
        wait(for: [resumed], timeout: 8)
        let blue = try centerPixel(of: app.screenshot())
        XCTAssertGreaterThan(Int(blue[2]), Int(max(blue[0], blue[1])) + 50)
    }

    private func centerPixel(of screenshot: XCUIScreenshot) throws -> [UInt8] {
        let image = try XCTUnwrap(screenshot.image.cgImage)
        let center = CGRect(x: image.width / 2, y: image.height / 2, width: 1, height: 1)
        let pixel = try XCTUnwrap(image.cropping(to: center))
        var bytes = [UInt8](repeating: 0, count: 4)
        try bytes.withUnsafeMutableBytes { buffer in
            let space = try XCTUnwrap(CGColorSpace(name: CGColorSpace.sRGB))
            let context = try XCTUnwrap(CGContext(data: buffer.baseAddress, width: 1, height: 1,
                bitsPerComponent: 8, bytesPerRow: 4, space: space,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            context.draw(pixel, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        }
        return bytes
    }

    func testSceneDrag() {
        let app = XCUIApplication(bundleIdentifier: "dev.scoop.ios-probe.window")
        app.launchArguments = ["--scene-drag"]
        app.launch()
        let result = app.descendants(matching: .any).matching(identifier: "scene-drag").firstMatch
        XCTAssertTrue(result.waitForExistence(timeout: 5))
        let left = app.coordinate(withNormalizedOffset: CGVector(dx: 0.25, dy: 0.4))
        let right = app.coordinate(withNormalizedOffset: CGVector(dx: 0.75, dy: 0.5))
        left.press(forDuration: 0.15, thenDragTo: right)
        XCTAssertEqual(result.value as? String, "moved once")
        right.press(forDuration: 0.15, thenDragTo: left)
        XCTAssertEqual(result.value as? String, "rejected")
    }

    func testWindowAccessibility() {
        let app = XCUIApplication(bundleIdentifier: "dev.scoop.ios-probe.window")
        app.launchArguments = ["--accessibility", "--inspect"]
        app.launch()
        let button = app.buttons["open-example"]
        XCTAssertTrue(button.waitForExistence(timeout: 5))
        XCTAssertEqual(button.label, "Open example")
        XCTAssertEqual(button.value as? String, "1")
        XCTAssertEqual(button.frame.size, CGSize(width: 160, height: 52))
    }

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
