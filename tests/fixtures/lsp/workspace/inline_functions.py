"""Apply a single inline edit and check explicit/default argument bindings."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "inline-functions"
    manifest(project, "inline-functions")
    file = "src/inline.scoop"
    text = (
        "package test\n"
        "fun mark(number: Int): Int { println(number); return number }\n"
        "private fun sum(first: Int, second: Int = mark(first), third: Int): Int {\n"
        "    val subtotal = first + second\n"
        "    return subtotal + third\n"
        "}\n"
        'private fun step(ready: Boolean): Boolean { println("unexpected"); return ready }\n'
        "private fun early(ready: Boolean): Boolean {\n"
        "    if (ready) { return false }\n"
        "    return true\n"
        "}\n"
        "fun compute(): Int = sum(third = mark(3), first = mark(1))\n"
        "fun skip(): Boolean = false && step(true)\n"
        "fun reject(): Boolean = early(false)\n"
        "fun shadowed(): Int {\n"
        "    fun mark(number: Int): Int = 9\n"
        "    return sum(third = 3, first = 1)\n"
        "}\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def action(anchor, name):
        position = case.position(file, anchor, name)
        result = peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": position, "end": position},
                "context": {"diagnostics": [], "only": ["refactor.inline"]},
            },
        )
        return next((item for item in result if item["title"] == "Inline Function"), None)

    assert action("= early(false)", "early") is None
    assert action("return sum(third = 3", "sum") is None
    inline = action("= sum(third", "sum")
    assert inline is not None
    case.apply(inline["edit"], 1)
    revised = case.texts[file]
    assert "private fun sum(" in revised
    assert "fun compute(): Int = (when {" in revised
    assert (
        revised.index("val inlineArgument3Value: Int = mark(3)")
        < revised.index("val inlineArgument1Value: Int = mark(1)")
        < revised.index("val inlineArgument2Value: Int = mark(inlineArgument1Value)")
        < revised.index("val subtotal = inlineArgument1Value + inlineArgument2Value")
    )
    inline = action("&& step(true)", "step")
    assert inline is not None
    case.apply(inline["edit"], 1)
    assert "false && (when {" in case.texts[file]
    case.finish()
