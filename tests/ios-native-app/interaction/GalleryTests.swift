import XCTest

final class GalleryTests: XCTestCase {
    private let app = XCUIApplication(bundleIdentifier: "dev.scoop.gallery")
    private let entryPrefix = "gallery-entry-"

    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .portrait
        app.launch()
    }

    private func element(_ identifier: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: identifier).firstMatch
    }

    private func open(_ title: String) {
        let entry = app.buttons[entryPrefix + title]
        if !entry.isHittable {
            let search = element("gallery-search")
            XCTAssertTrue(search.waitForExistence(timeout: 10), app.debugDescription)
            let clear = app.buttons["gallery-clear-search"]
            if clear.exists { clear.tap() }
            search.tap()
            app.typeText(title)
        }
        XCTAssertTrue(entry.waitForExistence(timeout: 5), app.debugDescription)
        entry.tap()
    }

    func testGalleryNavigation() {
        XCTAssertTrue(element("gallery-search").waitForExistence(timeout: 15), app.debugDescription)
        open("Settings app")
        let back = app.buttons["gallery-back"]
        XCTAssertTrue(back.waitForExistence(timeout: 5), app.debugDescription)
        XCTAssertTrue(app.staticTexts["Profile settings"].exists)
        XCUIDevice.shared.press(.home)
        app.activate()
        XCTAssertTrue(app.staticTexts["Profile settings"].exists)
        back.tap()
        open("Toast")
        XCTAssertTrue(back.waitForExistence(timeout: 5), app.debugDescription)
        back.tap()
        app.buttons["Appearance"].tap()
        let styles = element("gallery-style")
        XCTAssertTrue(styles.isHittable, app.debugDescription)
        for name in ["macOS", "Fluent", "shadcn", "Material"] {
            styles.tap()
            let choice = app.buttons.matching(NSPredicate(format: "label == %@", name)).firstMatch
            XCTAssertTrue(choice.waitForExistence(timeout: 5), app.debugDescription)
            choice.tap()
        }
        app.buttons["Dark theme"].tap()
        XCTAssertTrue(app.buttons["Light theme"].exists)
        app.buttons["Switch to RTL"].tap()
        XCTAssertTrue(app.buttons["Switch to LTR"].exists)
        app.buttons["Reduce motion"].tap()
        XCTAssertTrue(app.buttons["Full motion"].exists)
    }

    func testGalleryCatalog() {
        XCTAssertTrue(element("gallery-search").waitForExistence(timeout: 15), app.debugDescription)
        var visited = Set<String>()
        var unchanged = 0
        let back = app.buttons["gallery-back"]
        while visited.count < 69 && unchanged < 4 {
            let start = visited.count
            let entries = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", entryPrefix))
            let identifiers = entries.allElementsBoundByIndex.compactMap { row in
                row.isHittable && row.frame.height > 18 ? row.identifier : nil
            }
            for id in identifiers where !visited.contains(id) {
                let row = app.buttons[id]
                guard row.isHittable else { continue }
                row.tap()
                XCTAssertTrue(back.waitForExistence(timeout: 6), "Opening \(id)\n\(app.debugDescription)")
                let title = String(id.dropFirst(entryPrefix.count))
                XCTAssertTrue(
                    app.staticTexts.matching(NSPredicate(format: "label == %@", title)).firstMatch.exists,
                    title)
                back.tap()
                XCTAssertTrue(entries.firstMatch.waitForExistence(timeout: 5))
                visited.insert(id)
            }
            if visited.count == 69 { break }
            unchanged = visited.count == start ? unchanged + 1 : 0
            let from = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.83))
            let to = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.44))
            from.press(forDuration: 0.12, thenDragTo: to)
        }
        XCTAssertEqual(visited.count, 69, visited.sorted().joined(separator: ", "))
    }

    private func reveal(_ target: XCUIElement, near x: CGFloat = 0.82) {
        for _ in 0..<8 {
            let window = app.windows.firstMatch
            let frame = window.frame
            let keyboard = app.keyboards.firstMatch
            let accessory = app.otherElements["SystemInputAssistantView"].firstMatch
            var bottom = frame.maxY
            if keyboard.exists { bottom = min(bottom, keyboard.frame.minY) }
            if accessory.exists { bottom = min(bottom, accessory.frame.minY) }
            if target.isHittable && target.frame.height >= 18 && target.frame.midY < bottom - 8 { return }
            let height = bottom - frame.minY
            let origin = window.coordinate(withNormalizedOffset: .zero)
            let from = origin.withOffset(CGVector(dx: x * frame.width, dy: height * 0.86))
            let to = origin.withOffset(CGVector(dx: x * frame.width, dy: height * 0.49))
            from.press(forDuration: 0.12, thenDragTo: to)
        }
        XCTAssertTrue(target.isHittable, app.debugDescription)
    }

    private func replace(_ identifier: String, with text: String) {
        let field = element(identifier)
        reveal(field)
        field.tap()
        let editor = app.textViews[identifier]
        if !editor.waitForExistence(timeout: 1) {
            reveal(field)
            field.tap()
        }
        XCTAssertTrue(editor.waitForExistence(timeout: 3), app.debugDescription)
        if let current = field.value as? String, !current.isEmpty {
            app.typeKey("a", modifierFlags: .command)
        }
        app.typeText(text)
    }

    func testGalleryConversation() {
        open("Conversation app")
        let composer = element("conversation-composer")
        reveal(composer)
        composer.tap()
        app.typeText("Review the draft")
        let send = app.buttons["Send"]
        XCTAssertTrue(send.isEnabled)
        send.tap()
        let complete = app.staticTexts["Reply complete."]
        XCTAssertTrue(complete.waitForExistence(timeout: 10), app.debugDescription)
        XCTAssertFalse(app.buttons["Stop reply"].isEnabled)
    }

    func testGalleryLeavingConversation() {
        open("Conversation app")
        replace("conversation-composer", with: "Interrupt this reply")
        app.buttons["Send"].tap()
        app.buttons[entryPrefix + "Settings app"].tap()
        app.buttons[entryPrefix + "Conversation app"].tap()
        let stop = app.buttons["Stop reply"]
        XCTAssertTrue(stop.waitForExistence(timeout: 5), app.debugDescription)
        XCTAssertFalse(stop.isEnabled)
        XCTAssertFalse(app.staticTexts["Reply complete."].waitForExistence(timeout: 3))
    }

    func testGalleryTreeDrag() {
        open("Tree")
        let collapse = app.buttons["Collapse branches"]
        reveal(collapse)
        collapse.tap()
        let row = app.descendants(matching: .any)
        let notes = row.matching(NSPredicate(format: "label == 'notes.scoop'")).firstMatch
        let archive = row.matching(NSPredicate(format: "label == 'archive'")).firstMatch
        reveal(notes)
        XCTAssertTrue(archive.isHittable, app.debugDescription)
        let withinRow = CGVector(dx: 0.4, dy: 0.5)
        notes.coordinate(withNormalizedOffset: withinRow).tap()
        notes.coordinate(withNormalizedOffset: withinRow)
            .press(forDuration: 0.7, thenDragTo: archive.coordinate(withNormalizedOffset: withinRow))
        let moved = app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS '1 selected' AND label CONTAINS '1 moves'")
        ).firstMatch
        XCTAssertTrue(moved.waitForExistence(timeout: 5), app.debugDescription)
    }

    func testGalleryTimers() {
        open("Toast")
        let save = app.buttons["Save workspace"]
        reveal(save)
        save.tap()
        let completed = app.staticTexts["Task completed"]
        XCTAssertTrue(completed.waitForExistence(timeout: 8), app.debugDescription)
        open("Tree")
        let remote = app.buttons["Load Remote"]
        reveal(remote)
        remote.tap()
        let failure = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'Use its retry arrow'"))
            .firstMatch
        XCTAssertTrue(failure.waitForExistence(timeout: 6), app.debugDescription)
        remote.tap()
        let loaded = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH 'Loaded ' AND label CONTAINS 'Remote'")
        ).firstMatch
        XCTAssertTrue(loaded.waitForExistence(timeout: 6), app.debugDescription)
    }

    func testGalleryForms() {
        open("Settings app")
        replace("settings-name", with: "Scoop Gallery")
        replace("settings-email", with: "gallery@example.com")
        replace("settings-code", with: "123456")
        let save = app.buttons["Save settings"]
        reveal(save)
        save.tap()
        let saved = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Saved 1 time'"))
            .firstMatch
        XCTAssertTrue(saved.waitForExistence(timeout: 5), app.debugDescription)

        open("Workspace app")
        replace("workspace-draft", with: "Gallery workspace draft")
        open("Conversation app")
        open("Workspace app")
        XCTAssertEqual(element("workspace-draft").value as? String, "Gallery workspace draft")
    }

    func testGalleryChineseSettings() {
        open("Settings app")
        let name = element("settings-name")
        reveal(name)
        name.tap()
        app.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: 5))
        let keyboard = app.keyboards.firstMatch
        for letter in "nihao" {
            keyboard.keys.matching(NSPredicate(format: "label ==[c] %@", String(letter))).firstMatch.tap()
        }
        let candidate = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "你好"))
            .firstMatch
        XCTAssertTrue(candidate.waitForExistence(timeout: 5), app.debugDescription)
        candidate.tap()
        XCTAssertEqual(name.value as? String, "你好")
        replace("settings-email", with: "gallery@example.com")
        replace("settings-code", with: "123456")
        let save = app.buttons["Save settings"]
        reveal(save, near: 0.035)
        save.tap()
        XCTAssertTrue(
            app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Saved 1 time'")).firstMatch
                .waitForExistence(timeout: 5), app.debugDescription)
    }

    func testGalleryPaymentUndo() {
        open("Data app")
        let action = app.buttons.matching(
            NSPredicate(format: "identifier BEGINSWITH 'payment-actions-' AND enabled == true")
        ).firstMatch
        let amount = app.buttons["Amount"]
        XCTAssertTrue(amount.waitForExistence(timeout: 5))
        if !action.isHittable {
            let start = amount.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).withOffset(
                CGVector(dx: 0, dy: 90))
            start.press(forDuration: 0.12, thenDragTo: start.withOffset(CGVector(dx: -320, dy: 0)))
        }
        XCTAssertTrue(action.isHittable, app.debugDescription)
        action.tap()
        let remove = app.descendants(matching: .any).matching(
            NSPredicate(format: "label == 'Delete payment'")
        ).firstMatch
        XCTAssertTrue(remove.waitForExistence(timeout: 5), app.debugDescription)
        remove.tap()
        XCTAssertTrue(app.staticTexts["Delete this payment?"].waitForExistence(timeout: 5))
        app.buttons["Delete payment"].tap()
        let undo = app.buttons["Undo"]
        XCTAssertTrue(undo.waitForExistence(timeout: 5), app.debugDescription)
        undo.tap()
        XCTAssertTrue(
            app.staticTexts.matching(NSPredicate(format: "label ENDSWITH 'restored with its note.'"))
                .firstMatch.exists)
    }

    private func enterFullScreen(_ window: XCUIElement) {
        let top = window.coordinate(withNormalizedOffset: .zero)
        let system = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let controls = system.otherElements.matching(identifier: "window-controls:dev.scoop.gallery")
            .firstMatch
        if controls.exists {
            controls.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        } else {
            top.withOffset(CGVector(dx: 40, dy: window.frame.minY == 0 ? 54 : 30)).tap()
        }
        let zoom = controls.buttons["Zoom-button"].firstMatch
        if zoom.waitForExistence(timeout: 1) {
            zoom.tap()
        } else {
            top.withOffset(CGVector(dx: 104, dy: window.frame.minY == 0 ? 54 : 30)).tap()
        }
        let wide = XCTNSPredicateExpectation(
            predicate: NSPredicate { _, _ in window.frame.width > 720 }, object: window)
        XCTAssertEqual(XCTWaiter.wait(for: [wide], timeout: 5), .completed)
    }

    func testGalleryResizedWindow() {
        let window = app.windows.firstMatch
        if window.frame.width < 720 { self.enterFullScreen(window) }
        open("Settings app")
        let original = window.frame
        XCTAssertGreaterThan(original.width, 720)
        let edge = CGVector(dx: -6, dy: -6)
        let corner = window.coordinate(withNormalizedOffset: CGVector(dx: 1, dy: 1)).withOffset(edge)
        let finish = corner.withOffset(CGVector(dx: -original.width * 0.42, dy: -150))
        corner.press(forDuration: 0.15, thenDragTo: finish)
        addTeardownBlock {
            if window.frame.width < 720 { self.enterFullScreen(window) }
        }
        let back = app.buttons["gallery-back"]
        XCTAssertTrue(back.waitForExistence(timeout: 8), app.debugDescription)
        XCTAssertLessThan(window.frame.width, 720)
        XCTAssertTrue(app.staticTexts["Profile settings"].exists)
        back.tap()
        XCTAssertTrue(element("gallery-search").waitForExistence(timeout: 5), app.debugDescription)
    }

    func testGalleryBesidePhotos() {
        let window = app.windows.firstMatch
        if window.frame.width < 720 { enterFullScreen(window) }
        open("Settings app")
        let system = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let corner = window.coordinate(withNormalizedOffset: CGVector(dx: 1, dy: 1))
            .withOffset(CGVector(dx: -6, dy: -6))
        corner.press(forDuration: 0.15, thenDragTo: corner.withOffset(CGVector(dx: -350, dy: -150)))
        let windowControls = window.coordinate(withNormalizedOffset: .zero).withOffset(
            CGVector(dx: 40, dy: 30))
        windowControls.press(forDuration: 0.8)
        let sideBySide = system.buttons.containing(
            .image, identifier: "inset.filled.lefthalf.righthalf.rectangle"
        ).firstMatch
        XCTAssertTrue(sideBySide.waitForExistence(timeout: 3))
        sideBySide.tap()
        let other = XCUIApplication(bundleIdentifier: "com.apple.mobileslideshow")
        let photos = system.icons.matching(NSPredicate(format: "label IN %@", ["Photos", "照片"])).firstMatch
        if photos.waitForExistence(timeout: 4) && photos.isHittable { photos.tap() }
        addTeardownBlock {
            if window.frame.width < 720 { self.enterFullScreen(window) }
        }
        XCTAssertTrue(app.buttons["gallery-back"].waitForExistence(timeout: 8), app.debugDescription)
        XCTAssertTrue(app.staticTexts["Profile settings"].exists)
        XCTAssertTrue(other.windows.firstMatch.waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(window.frame.maxX, other.windows.firstMatch.frame.minX + 2)
        XCTAssertLessThan(window.frame.width, 720)
        app.buttons["gallery-back"].tap()
        XCTAssertTrue(element("gallery-search").waitForExistence(timeout: 5))
    }

    func testGalleryWide() {
        XCTAssertTrue(element("gallery-search").waitForExistence(timeout: 15), app.debugDescription)
        open("Settings app")
        XCTAssertTrue(app.staticTexts["Profile settings"].waitForExistence(timeout: 5))
        XCTAssertTrue(element("gallery-search").isHittable)
        XCTAssertFalse(app.buttons["gallery-back"].exists)
        open("Conversation app")
        XCTAssertTrue(
            app.staticTexts["Project conversation"].waitForExistence(timeout: 5), app.debugDescription)
    }
}
