"""Apply constants, computed properties and defaulted parameter extraction."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "extract-declarations"
    manifest(project, "extract-declarations")
    file = "src/extract.scoop"
    text = (
        "package test\n"
        "fun five(): Int { return 2 + 3 }\n"
        "class Model(public var ready: Boolean) {\n"
        "    fun read(): Boolean = ready\n"
        "    fun depends(input: Boolean): Boolean { return input }\n"
        "}\n"
        "private fun keep(flag: Boolean): Boolean { return flag }\n"
        "fun useExisting(): Boolean = keep(true)\n"
        "private fun effect(): Boolean { return keep(false) }\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def action(anchor, selected, title):
        start = case.position(file, anchor, selected)
        end = {**start, "character": start["character"] + len(selected)}
        result = peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": start, "end": end},
                "context": {"diagnostics": [], "only": ["refactor.extract"]},
            },
        )
        return next((item for item in result if item["title"] == title), None)

    constant = action("return 2 + 3", "2 + 3", "Extract Constant")
    assert constant is not None
    case.apply(constant["edit"], 1)
    assert "private const val extractedConstant: Int = 2 + 3" in case.texts[file]
    assert "return extractedConstant" in case.texts[file]
    assert action("return keep(false)", "keep(false)", "Extract Constant") is None

    assert action("return input", "input", "Extract Property") is None
    computed = action("fun read(): Boolean = ready", "ready", "Extract Property")
    assert computed is not None
    case.apply(computed["edit"], 1)
    assert "private val extractedProperty: Boolean get() = ready" in case.texts[file]
    assert "fun read(): Boolean = extractedProperty" in case.texts[file]

    assert action("return keep(false)", "keep(false)", "Extract Parameter") is None
    parameter = action("return flag", "flag", "Extract Parameter")
    assert parameter is not None
    case.apply(parameter["edit"], 1)
    assert "keep(flag: Boolean, extractedParameter: Boolean = flag)" in case.texts[file]
    assert "return extractedParameter" in case.texts[file]
    assert "fun useExisting(): Boolean = keep(true)" in case.texts[file]
    case.finish()
