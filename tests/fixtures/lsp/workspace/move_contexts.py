"""Move functions across namespace and Cone boundaries with their usages."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "move-contexts"
    manifest(project, "move-contexts", (("target", "dependency"),))
    manifest(project / "dependency", "target")
    source = "src/original.scoop"
    destination = "src/target.scoop"
    callers = "src/callers.scoop"
    dependency = "dependency/src/target.scoop"
    case = RenameWorkspace(
        peer,
        project,
        {
            source: (
                "package test.fromPackage\n"
                "fun helper(flag: Boolean): Boolean = flag\n"
                "/// This documentation follows the declaration.\n"
                "public fun migrate(flag: Boolean): Boolean = helper(flag)\n"
                "fun localCaller(): Boolean = migrate(true)\n"
                "public fun exportable(flag: Boolean): Boolean = flag\n"
                "fun coneCaller(): Boolean = exportable(flag = true)\n"
                "public fun stays(flag: Boolean): Boolean = helper(flag)\n"
            ),
            destination: "package test.toPackage\nfun present(): Boolean = true\n",
            callers: (
                "package test.callers\n"
                "import test.fromPackage.migrate as viaAlias\n"
                "fun aliasCaller(): Boolean = viaAlias(false)\n"
                "fun functionReference(): (Boolean) -> Boolean = ::viaAlias\n"
            ),
            dependency: "package test.dependencyPackage\nfun targetExists(): Boolean = true\n",
        },
    )
    case.clean()

    def move(name, target):
        return peer.request(
            "scoop/moveDeclaration",
            {
                "textDocument": {"uri": case.paths[source].as_uri()},
                "position": case.position(source, f"public fun {name}(", name),
                "targetUri": case.paths[target].as_uri(),
            },
        )

    edit = move("migrate", destination)
    assert edit is not None
    case.apply(edit, 5)
    assert "import test.toPackage.migrate" in case.texts[source]
    assert "import test.fromPackage.helper" in case.texts[destination]
    assert "This documentation follows the declaration." in case.texts[destination]
    assert "import test.toPackage.migrate as viaAlias" in case.texts[callers]
    for path, anchor, name in (
        (source, "= migrate(true)", "migrate"),
        (callers, "= viaAlias(false)", "viaAlias"),
        (callers, "::viaAlias", "viaAlias"),
    ):
        location = case.query(path, anchor, name, "definition")
        assert location["uri"] == case.paths[destination].as_uri()

    assert move("stays", dependency) is None
    edit = move("exportable", dependency)
    assert edit is not None
    case.apply(edit, 3)
    assert "import test.dependencyPackage.exportable" in case.texts[source]
    assert "public fun exportable" in case.texts[dependency]
    location = case.query(source, "= exportable(flag = true)", "exportable", "definition")
    assert location["uri"] == case.paths[dependency].as_uri()
    case.finish()


def check_reexport(peer, root):
    project = root / "move-reexport"
    manifest(project, "move-reexport", (("provider", "provider"),))
    manifest(project / "provider", "provider")
    source = "provider/src/old.scoop"
    destination = "provider/src/new.scoop"
    facade = "src/facade.scoop"
    case = RenameWorkspace(
        peer,
        project,
        {
            source: "package test.old\npublic fun send(flag: Boolean): Boolean = flag\n",
            destination: "package test.new\nfun marker(): Boolean = false\n",
            facade: (
                "package test.facade\n"
                "public import test.old.send as stable\n"
                "fun use(): Boolean = stable(true)\n"
            ),
        },
    )
    case.clean()
    edit = peer.request(
        "scoop/moveDeclaration",
        {
            "textDocument": {"uri": case.paths[source].as_uri()},
            "position": case.position(source, "public fun send", "send"),
            "targetUri": case.paths[destination].as_uri(),
        },
    )
    assert edit is not None
    case.apply(edit, 3)
    assert "public import test.new.send as stable" in case.texts[facade]
    assert "fun use(): Boolean = stable(true)" in case.texts[facade]
    assert (
        case.query(facade, "= stable(true)", "stable", "definition")["uri"]
        == case.paths[destination].as_uri()
    )
    case.finish()
