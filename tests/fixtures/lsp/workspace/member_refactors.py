"""Exercise each inheritance refactoring through the stdio editor surface."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "member-refactors"
    manifest(project, "member-refactors")
    file = "src/members.scoop"
    text = (
        "package test\n"
        "class Sample {\n"
        "    public fun measure(value: Int): Int = value + 1\n"
        "}\n"
        "class ExtractFrom(val state: Int) {\n"
        "    /** Doubles the input. */\n"
        "    fun double(value: Int): Int = value * 2\n"
        "    fun stored(): Int = state\n"
        "}\n"
        "open class PullTarget()\n"
        "class PullChild : PullTarget() {\n"
        "    fun up(value: Int): Int = value + 2\n"
        "}\n"
        "open class PushRoot() {\n"
        "    fun down(value: Int): Int = value + 3\n"
        "    fun retained(): Boolean = true\n"
        "}\n"
        "class PushLeaf : PushRoot() {}\n"
        "fun measure(value: Sample): Int = value.measure(1)\n"
        "fun doubled(value: ExtractFrom): Int = value.double(2)\n"
        "fun lifted(value: PullChild): Int = value.up(3)\n"
        "fun pushed(value: PushLeaf): Int = value.down(4)\n"
        "fun usesBase(value: PushRoot): Boolean = value.retained()\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def action(anchor, name, title, kind="extract"):
        position = case.position(file, anchor, name)
        result = peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "range": {"start": position, "end": position},
                "context": {"diagnostics": [], "only": [f"refactor.{kind}"]},
            },
        )
        return next((item for item in result if item["title"] == title), None)

    interface = action("class Sample", "Sample", "Extract Interface")
    assert interface is not None
    case.apply(interface["edit"], 3)
    assert "interface ExtractedInterface" in case.texts[file]
    assert "class Sample : ExtractedInterface" in case.texts[file]
    assert "public final override fun measure" in case.texts[file]
    contract = case.position(file, "public fun measure(value", "measure")
    implementations = peer.request(
        "textDocument/implementation",
        {"textDocument": {"uri": case.paths[file].as_uri()}, "position": contract},
    )
    assert len(implementations) == 1
    assert implementations[0]["range"]["start"] == case.position(
        file, "override fun measure(value", "measure"
    )

    assert action("fun stored()", "stored", "Extract Superclass") is None
    base = action("fun double(value", "double", "Extract Superclass")
    assert base is not None
    case.apply(base["edit"], 3)
    assert "open class ExtractedBase()" in case.texts[file]
    assert "class ExtractFrom(val state: Int) : ExtractedBase()" in case.texts[file]
    assert case.texts[file].index("/** Doubles the input. */") < case.texts[file].index(
        "class ExtractFrom"
    )

    up = action("fun up(value", "up", "Pull Members Up", "rewrite")
    assert up is not None
    case.apply(up["edit"], 2)
    assert "open class PullTarget() {\n    fun up(" in case.texts[file]
    assert "class PullChild : PullTarget() {\n}" in case.texts[file]

    assert action("fun retained()", "retained", "Push Members Down", "rewrite") is None
    down = action("fun down(value", "down", "Push Members Down", "rewrite")
    assert down is not None
    case.apply(down["edit"], 2)
    assert "class PushLeaf : PushRoot() {\n    fun down(" in case.texts[file]
    assert "open class PushRoot() {\n    fun retained()" in case.texts[file]
    case.finish()
