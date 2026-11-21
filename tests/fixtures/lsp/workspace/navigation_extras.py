"""Use real source bindings for control flow, highlights and test navigation."""

from rename_support import RenameWorkspace, manifest


def check(peer, root, restore="production"):
    project = root / "navigation-extras"
    manifest(project, "navigation-extras")
    manifest_path = project / "Cone.toml"
    with manifest_path.open("a") as handle:
        handle.write('\n[test-dependencies]\n"scoop:scoop.test" = "0.1.0"\n')
    file = "src/targets.scoop"
    text = (
        "package test\n"
        "fun jumps(flag: Boolean): Boolean {\n"
        "    outer@ while (flag) {\n"
        "        while (flag) { break }\n"
        "        if (flag) { continue@outer }\n"
        "        break@outer\n"
        "    }\n"
        "    return flag\n"
        "}\n"
        "fun callback(): () -> Boolean = done@ { return@done true }\n"
        "fun anonymous(): () -> Boolean = fun(): Boolean { return true }\n"
        "fun highlight(flag: Boolean): Boolean {\n"
        "    val selected = flag\n"
        "    if (flag) { val selected = false; selected.toString() }\n"
        "    return selected\n"
        "}\n"
    )
    test_file = "tests/targets.scoop"
    test = (
        "package test\n"
        "import scoop.test.Test\n"
        "@Test class NavigationCases {\n"
        "    @Test fun exercised() { jumps(false) }\n"
        "}\n"
    )
    case = RenameWorkspace(peer, project, {file: text, test_file: test})

    def goto(anchor, name, target_line, target):
        result = case.query(file, anchor, name, "definition")
        assert result["uri"] == case.paths[file].as_uri(), result
        target_column = text.splitlines()[target_line].index(target)
        assert result["range"]["start"] == {
            "line": target_line,
            "character": target_column,
        }, result

    goto("while (flag) { break }", "break", 3, "while")
    goto("continue@outer", "continue", 2, "outer")
    goto("break@outer", "outer", 2, "outer")
    goto("return flag", "return", 1, "jumps")
    goto("return@done true", "return", 9, "done")
    goto("return true", "return", 10, "fun():")

    highlights = case.query(file, "return selected", "selected", "documentHighlight")
    assert [item["range"]["start"]["line"] for item in highlights] == [12, 14], highlights

    def tests():
        return peer.request(
            "scoop/relatedTests",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "position": case.position(file, "fun jumps", "jumps"),
            },
        )

    def configure(purpose):
        peer.notify(
            "workspace/didChangeConfiguration",
            {
                "settings": {
                    "scoop": {
                        "target": peer.target,
                        "sysroot": str(peer.sysroot),
                        "packageManager": str(peer.package_manager),
                        "packageCacheDir": str(peer.cache),
                        "purpose": purpose,
                    }
                }
            },
        )

    try:
        configure("production")
        assert tests() == []
        configure("test")
        [usage] = tests()
        assert usage["uri"] == case.paths[test_file].as_uri(), usage
        assert usage["range"]["start"] == {"line": 3, "character": 28}, usage
    finally:
        configure(restore)
        case.finish()
