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
