"""Apply selection-based edits against the real project's checked source."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "extract-local"
    manifest(project, "extract-local")
    file = "src/selection.scoop"
    text = (
        "package test\n"
        "fun evaluate(flag: Boolean): Boolean = flag\n"
        "fun combine(first: Boolean, second: Boolean): Boolean = first && second\n"
        "fun consume(flags: Array<Boolean>): Boolean = true\n"
        "fun values(flag: Boolean): Boolean {\n"
        "    val extracted = flag\n"
        "    val saved = evaluate(/* 🌟 */ extracted)\n"
        "    return evaluate(saved)\n"
        "}\n"
        "fun call(): Boolean { return consume([]) }\n"
        "fun order(): Boolean { return combine(evaluate(true), evaluate(false)) }\n"
        "fun short(flag: Boolean): Boolean { return flag && evaluate(false) }\n"
        "fun statements(flag: Boolean) {\n"
        "    val first = evaluate(flag)\n"
        "    // Evaluate the next call after the first.\n"
        "    evaluate(first)\n"
        "}\n"
        "fun outside(flag: Boolean): Boolean {\n"
        "    val escaped = evaluate(flag)\n"
        "    evaluate(escaped)\n"
        "    return escaped\n"
        "}\n"
        "fun writeBack(flag: Boolean): Boolean {\n"
        "    var current = flag\n"
        "    current = evaluate(false)\n"
        "    return current\n"
        "}\n"
        "fun atSite(site: SourceLocation = currentSourceLocation()): SourceLocation = site\n"
        "fun callerSite(): SourceLocation { return atSite() }\n"
    ).replace("\n", "\r\n")
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def actions(anchor, selected, kind="refactor", title="Extract Local Variable"):
        start = case.position(file, anchor, selected)
        lines = selected.splitlines()
        end = {
            "line": start["line"] + len(lines) - 1,
            "character": (start["character"] if len(lines) == 1 else 0)
            + len(lines[-1].encode("utf-16-le")) // 2,
        }
        result = peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": start, "end": end},
                "context": {"diagnostics": [], "only": [kind]},
            },
        )
        return [action for action in result if action["title"] == title]

    assert actions("return flag && evaluate(false)", "evaluate(false)") == []
    assert actions("combine(evaluate(true), evaluate(false))", "evaluate(false)") == []
    assert (
        actions(
            "val saved = evaluate(/* 🌟 */ extracted)", "evaluate(/* 🌟 */ extracted)", "quickfix"
        )
        == []
    )
    [action] = actions("val saved = evaluate(/* 🌟 */ extracted)", "evaluate(/* 🌟 */ extracted)")
    assert action["kind"] == "refactor.extract"
    resolved = peer.request("codeAction/resolve", action)
    case.apply(resolved["edit"], 1)
    assert (
        "val extracted2 = evaluate(/* 🌟 */ extracted)\r\n    val saved = extracted2"
        in case.texts[file]
    )
    [action] = actions("return evaluate(saved)", "evaluate(saved)")
    case.apply(action["edit"], 1)
    assert "return extracted3" in case.texts[file]
    [action] = actions("consume([])", "[]")
    case.apply(action["edit"], 1)
    assert "val extracted4: Array<Boolean> = []; return consume(extracted4)" in case.texts[file]
    [action] = actions("combine(evaluate(true), evaluate(false))", "evaluate(true)")
    extract_function = "Extract Function"
    [function] = actions(
        "return flag && evaluate(false)", "evaluate(false)", title=extract_function
    )
    case.apply(function["edit"], 1)
    assert "fun extractedFunction()" in case.texts[file]
    assert " = evaluate(false); return flag && extractedFunction()" in case.texts[file]
    statements = (
        "val first = evaluate(flag)\r\n"
        "    // Evaluate the next call after the first.\r\n"
        "    evaluate(first)"
    )
    [function] = actions(statements, statements, title=extract_function)
    case.apply(function["edit"], 1)
    assert (
        "fun extractedFunction2() {\r\n"
        "        val first = evaluate(flag)\r\n"
        "        // Evaluate the next call after the first.\r\n"
        "        evaluate(first)\r\n"
        "    }\r\n"
        "    extractedFunction2()"
    ) in case.texts[file]
    escaped = "val escaped = evaluate(flag)\r\n    evaluate(escaped)"
    assert actions(escaped, escaped, title=extract_function) == []
    outside = "current = evaluate(false)"
    assert actions(outside, outside, title=extract_function) == []
    assert actions("return current", "return current", title=extract_function) == []
    assert actions("return atSite()", "atSite()") == []
    assert actions("return atSite()", "atSite()", title=extract_function) == []
    case.change(file, case.texts[file] + "\r\n")
    pending = peer.send_request("codeAction/resolve", action)
    peer.response(pending, error_code=-32801)
    case.finish()
