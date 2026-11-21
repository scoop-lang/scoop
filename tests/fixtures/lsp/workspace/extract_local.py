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
    ).replace("\n", "\r\n")
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def actions(anchor, selected, kind="refactor"):
        start = case.position(file, anchor, selected)
        end = {**start, "character": start["character"] + len(selected.encode("utf-16-le")) // 2}
        return peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": start, "end": end},
                "context": {"diagnostics": [], "only": [kind]},
            },
        )

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
    case.change(file, case.texts[file] + "\r\n")
    pending = peer.send_request("codeAction/resolve", action)
    peer.response(pending, error_code=-32801)
    case.finish()
