"""Apply local refactorings and inspect usage locations that block deletion."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "refactor-local"
    manifest(project, "refactor-local")
    file = "src/refactor.scoop"
    text = (
        "package test\n"
        "/// Remove this documentation with the unused declaration.\n"
        "private fun unused(flag: Boolean): Boolean = flag\n"
        "private fun used(): Boolean = true\n"
        "fun direct(): Boolean = used()\n"
        "fun asValue(): () -> Boolean = ::used\n"
        "fun inlining(input: Boolean): Boolean {\n"
        "    val local = true\n"
        "    val copied = input\n"
        "    return local && copied && local\n"
        "}\n"
        "fun mutable(): Boolean {\n"
        "    var evolving = true\n"
        "    val captured = evolving\n"
        "    evolving = false\n"
        "    return captured\n"
        "}\n"
        "fun tuple(input: (Boolean, Boolean)): Boolean = true\n"
        "fun <T> preserve(value: T): T {\n"
        "    val stored: T = value\n"
        "    return stored\n"
        "}\n"
        "private fun omitted(flag: Boolean = used()): Boolean = flag\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def actions(anchor, name, kind, select=False):
        start = case.position(file, anchor, name)
        end = {**start, "character": start["character"] + (len(name) if select else 0)}
        return peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": start, "end": end},
                "context": {"diagnostics": [], "only": [kind]},
            },
        )

    rewrite = "refactor.rewrite"
    [blockage] = actions("private fun used()", "used", rewrite)
    assert "edit" not in blockage, blockage
    command = blockage["command"]
    assert command["command"] == "scoop.showSafeDeleteUsages", command
    [argument] = command["arguments"]
    assert sorted(location["range"]["start"]["line"] for location in argument["locations"]) == [
        4,
        5,
        len(text.splitlines()) - 1,
    ]
    [action] = actions("private fun unused(", "unused", rewrite)
    assert action["title"] == "Safe Delete Function"
    case.apply(action["edit"], 1)
    assert "private fun unused" not in case.texts[file]
    assert "Remove this documentation" not in case.texts[file]

    inline = "refactor.inline"
    assert actions("val captured =", "captured", inline) == []
    [action] = actions("return local &&", "local", inline)
    assert action["title"] == "Inline Local Variable"
    resolved = peer.request("codeAction/resolve", action)
    case.apply(resolved["edit"], 3)
    assert "return (true) && copied && (true)" in case.texts[file]
    [action] = actions("val copied = input", "copied", inline)
    case.apply(action["edit"], 2)
    assert "return (true) && (input) && (true)" in case.texts[file]

    extract = "refactor.extract"
    [action] = actions("input: (Boolean, Boolean)", "(Boolean, Boolean)", extract, select=True)
    assert action["title"] == "Introduce Type Alias"
    case.apply(action["edit"], 1)
    assert "private typealias ExtractedType = (Boolean, Boolean)" in case.texts[file]
    assert "fun tuple(input: ExtractedType)" in case.texts[file]
    [action] = actions("stored: T = value", "T", extract, select=True)
    case.apply(action["edit"], 1)
    assert "typealias ExtractedType2 = T\n    val stored: ExtractedType2" in case.texts[file]
    case.finish()
