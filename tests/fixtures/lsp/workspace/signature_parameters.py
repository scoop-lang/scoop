"""Apply edits to parameter lists, defaults and known call sites."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "signature-parameters"
    manifest(project, "signature-parameters")
    file = "src/parameters.scoop"
    text = (
        "package test\n"
        "private fun token(): Boolean = false\n"
        "fun stillCalls(): Boolean = token()\n"
        "private fun choose(first: Boolean, unused: Boolean = false, second: Boolean = true): Boolean = first && second\n"
        "fun supplied(): Boolean = choose(true, false, false)\n"
        "fun named(): Boolean = choose(first = true, unused = false, second = false)\n"
        "fun omitted(): Boolean = choose(first = true)\n"
        "private fun withReference(flag: Boolean): Boolean = flag\n"
        "fun makeReference(): (Boolean) -> Boolean = ::withReference\n"
        "private fun effectful(flag: Boolean): Boolean = true\n"
        "fun mustEvaluate(): Boolean = effectful(token())\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def edit(target, **operation):
        params = {
            "textDocument": {"uri": case.paths[file].as_uri()},
            "position": case.position(file, f"private fun {target}(", target),
        }
        prepared = peer.request("scoop/prepareChangeSignature", params)
        assert prepared is not None
        return peer.request(
            "scoop/changeSignature",
            {**params, "revision": prepared["revision"], "edit": operation},
        )

    assert edit("effectful", kind="removeUnused", parameter=0) is None
    assert edit("choose", kind="removeUnused", parameter=0) is None
    assert edit("choose", kind="setDefault", parameter=2, expression=None) is None
    assert (
        edit(
            "token",
            kind="addDefault",
            name="extra",
            type="Boolean",
            expression="false, other: Boolean",
        )
        is None
    )

    added = edit("token", kind="addDefault", name="extra", type="Boolean", expression="false")
    assert added is not None
    case.apply(added, 1)
    assert "token(extra: Boolean = false): Boolean" in case.texts[file]
    assert "fun stillCalls(): Boolean = token()" in case.texts[file]

    removed = edit("choose", kind="removeUnused", parameter=1)
    assert removed is not None
    case.apply(removed, 3)
    assert "choose(first: Boolean, second: Boolean = true)" in case.texts[file]
    assert "choose(true, false)" in case.texts[file]
    assert "choose(first = true, second = false)" in case.texts[file]
    assert "choose(first = true)" in case.texts[file]

    changed = edit("choose", kind="setDefault", parameter=1, expression="!first")
    assert changed is not None
    case.apply(changed, 1)
    assert "choose(first: Boolean, second: Boolean = !first)" in case.texts[file]

    added = edit("withReference", kind="setDefault", parameter=0, expression="((true))")
    assert added is not None
    case.apply(added, 1)
    assert "withReference(flag: Boolean = ((true)))" in case.texts[file]
    assert "::withReference" in case.texts[file]
    cleared = edit("withReference", kind="setDefault", parameter=0, expression=None)
    assert cleared is not None
    case.apply(cleared, 1)
    assert "withReference(flag: Boolean)" in case.texts[file]
    case.finish()


def check_methods(peer, root):
    project = root / "signature-methods"
    manifest(project, "signature-methods")
    file = "src/methods.scoop"
    text = (
        "package methods\n"
        "interface Checker { fun check(flag: Boolean, limit: Int): Boolean }\n"
        "open class Base : Checker {\n"
        "    override fun check(enabled: Boolean, count: Int): Boolean = enabled && count == 1\n"
        "}\n"
        "class Child : Base() {\n"
        "    override fun check(on: Boolean, number: Int): Boolean = super.check(on, number)\n"
        "}\n"
        "fun viaInterface(checker: Checker): Boolean = checker.check(true, 1)\n"
        "fun viaChild(child: Child): Boolean = child.check(true, 1)\n"
        "private fun invoked(flag: Boolean, number: Int): Boolean = flag && number == 2\n"
        "fun viaReference(): Boolean = (::invoked)(true, 2)\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()
    params = {
        "textDocument": {"uri": case.paths[file].as_uri()},
        "position": case.position(file, "override fun check(on", "check"),
    }
    prepared = peer.request("scoop/prepareChangeSignature", params)
    assert prepared["parameters"] == ["on", "number"] and prepared["reorderOnly"]
    edit = peer.request(
        "scoop/changeSignature",
        {**params, "revision": prepared["revision"], "order": [1, 0]},
    )
    assert edit is not None
    case.apply(edit, 9)
    for fragment in (
        "fun check(limit: Int, flag: Boolean)",
        "fun check(count: Int, enabled: Boolean)",
        "fun check(number: Int, on: Boolean)",
        "super.check(enabled = on, count = number)",
        "checker.check(flag = true, limit = 1)",
        "child.check(on = true, number = 1)",
    ):
        assert fragment in case.texts[file], case.texts[file]

    params["position"] = case.position(file, "private fun invoked", "invoked")
    prepared = peer.request("scoop/prepareChangeSignature", params)
    assert prepared["parameters"] == ["flag", "number"] and not prepared["reorderOnly"]
    edit = peer.request(
        "scoop/changeSignature",
        {**params, "revision": prepared["revision"], "order": [1, 0]},
    )
    assert edit is not None
    case.apply(edit, 4)
    assert "private fun invoked(number: Int, flag: Boolean)" in case.texts[file]
    assert "fun viaReference(): Boolean = invoked(flag = true, number = 2)" in case.texts[file]
    case.finish()
