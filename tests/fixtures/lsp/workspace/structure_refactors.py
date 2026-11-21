"""Exercise versioned import edits, declaration moves and parameter reordering."""

from pathlib import Path

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "structure-refactors"
    manifest(project, "structure-refactors")
    artifact = Path(peer.sysroot) / "lib" / peer.target / "scoop.std.slib"
    with (project / "Cone.toml").open("a") as handle:
        handle.write(f'"scoop:scoop.std" = {{version = "=0.1.0", artifact = "{artifact}"}}\n')
    source = "src/original.scoop"
    destination = "src/destination.scoop"
    support = "src/support.scoop"
    case = RenameWorkspace(
        peer,
        project,
        {
            source: (
                "package test\n"
                "import test.support.echo as passthrough\n"
                "import test.support.ignored\n"
                "import test.support.ignored as oldName\n"
                "import test.support.*\n"
                "import test.unused.*\n"
                "public import scoop.time.Duration\n"
                "\n"
                "// Accompany the public declaration.\n"
                "/// Preserve this documentation.\n"
                "public fun moveMe(flag: Boolean): Boolean = passthrough(flag)\n"
                "fun useMoved(): Boolean = moveMe(true)\n"
                "fun useStar(): Boolean = onlyStar()\n"
                "fun useBlock(flag: Boolean): Boolean {\n"
                "    import test.support.echo as localEcho\n"
                "    import test.support.ignored as unusedBlock\n"
                "    return localEcho(flag)\n"
                "}\n"
                "fun location(): SourceLocation = currentSourceLocation()\n"
                "private fun choose(first: Boolean, second: Boolean = false): Boolean = first && second\n"
                "fun supplied(): Boolean = choose(true, false)\n"
                "fun useDefault(): Boolean = choose(true)\n"
                "private fun defaultDepends(first: Boolean, second: Boolean = first): Boolean = second\n"
                "fun useDependent(): Boolean = defaultDepends(true)\n"
                "private fun referenced(a: Boolean, b: Boolean): Boolean = a || b\n"
                "fun takeFunction(): (Boolean, Boolean) -> Boolean = ::referenced\n"
            ),
            destination: "package test\n\nfun alreadyHere(): Boolean = true\n",
            support: (
                "package test.support\n"
                "public fun echo(value: Boolean): Boolean = value\n"
                "public fun ignored(): Boolean = false\n"
                "public fun onlyStar(): Boolean = true\n"
            ),
            "src/unused.scoop": "package test.unused\nfun other(): Boolean = false\n",
        },
    )
    case.clean()

    def position(anchor, name):
        return {
            "textDocument": {"uri": case.paths[source].as_uri()},
            "position": case.position(source, anchor, name),
        }

    point = {"line": 0, "character": 0}
    [organization] = peer.request(
        "textDocument/codeAction",
        {
            "textDocument": {"uri": case.paths[source].as_uri()},
            "range": {"start": point, "end": point},
            "context": {"diagnostics": [], "only": ["source.organizeImports"]},
        },
    )
    assert organization["kind"] == "source.organizeImports"
    case.apply(organization["edit"], 4)
    assert "import test.support.ignored" not in case.texts[source]
    assert "import test.support.echo as passthrough" in case.texts[source]
    assert "import test.support.*" in case.texts[source]
    assert "import test.unused.*" not in case.texts[source]
    assert "as localEcho" in case.texts[source]
    assert "as unusedBlock" not in case.texts[source]
    assert "public import scoop.time.Duration" in case.texts[source]

    def move(anchor, name, target):
        return peer.request(
            "scoop/moveDeclaration",
            {**position(anchor, name), "targetUri": case.paths[target].as_uri()},
        )

    assert move("fun location()", "location", destination) is None
    assert move("fun moveMe(", "moveMe", support) is None
    moved = move("fun moveMe(", "moveMe", destination)
    assert moved is not None
    case.apply(moved, 3)
    assert "Preserve this documentation" not in case.texts[source]
    assert "Accompany the public declaration" not in case.texts[source]
    assert "import test.support.echo as passthrough" in case.texts[destination]
    assert "Preserve this documentation" in case.texts[destination]
    assert "Accompany the public declaration" in case.texts[destination]
    definition = case.query(source, "= moveMe(true)", "moveMe", "definition")
    assert definition["uri"] == case.paths[destination].as_uri()

    def signature(name):
        params = position(f"private fun {name}(", name)
        prepared = peer.request("scoop/prepareChangeSignature", params)
        return prepared, {**params, "order": [1, 0], "revision": prepared["revision"]}

    for name in ["defaultDepends", "referenced"]:
        _, params = signature(name)
        assert peer.request("scoop/changeSignature", params) is None
    prepared, stale = signature("choose")
    assert prepared["parameters"] == ["first", "second"]
    case.change(source, case.texts[source] + "\n")
    pending = peer.send_request("scoop/changeSignature", stale)
    peer.response(pending, error_code=-32801)
    _, params = signature("choose")
    changed = peer.request("scoop/changeSignature", params)
    assert changed is not None
    case.apply(changed, 4)
    assert "choose(second: Boolean = false, first: Boolean)" in case.texts[source]
    assert "choose(first = true, second = false)" in case.texts[source]
    assert "choose(first = true)" in case.texts[source]
    case.finish()
