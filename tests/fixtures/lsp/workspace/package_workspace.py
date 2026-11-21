"""Initialize missing locks and retain separate graphs, including cached Git sources."""

import subprocess
import sys
from pathlib import Path

from peer import Peer


def write(root, relative, text):
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def manifest(name, version="1.0.0"):
    return (
        f'schema = 1\n[cone]\ngroup = "test"\nname = "{name}"\n'
        f'version = "{version}"\nkind = "library"\n'
    )


def run(args):
    result = subprocess.run(args, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout.strip()


def check_mixed_purposes(server, package_manager, root, sysroot, target, cache):
    write(
        root,
        "Cone.toml",
        'schema = 1\n[workspace]\nmembers = ["app", "checks", "support"]\ndefault-members = ["app"]\n',
    )
    write(root, "app/Cone.toml", manifest("app"))
    write(root, "app/src/app.scoop", "fun application(): Boolean = true\n")
    write(
        root,
        "checks/Cone.toml",
        manifest("checks") + '[test-dependencies]\n"test:support" = {path = "../support"}\n',
    )
    write(root, "checks/src/package.scoop", "package checks\n")
    text = "package checks\nimport support.answer\nfun testScenario(): Boolean = answer()\n"
    source = write(root, "checks/tests/scenario.scoop", text)
    write(root, "support/Cone.toml", manifest("support"))
    support = write(
        root, "support/src/answer.scoop", "package support\npublic fun answer(): Boolean = true\n"
    )
    for purpose, member in (("production", "app"), ("test", "checks")):
        run(
            [
                package_manager,
                "resolve",
                str(root),
                "--offline",
                "-p",
                f"test:{member}",
                "--purpose",
                purpose,
                "--target",
                target,
                "--sysroot",
                sysroot,
                "--package-cache-dir",
                str(cache),
            ]
        )
    lock = root / "Cone.lock"
    before = (lock.read_bytes(), lock.stat().st_mtime_ns)
    options = {
        "target": target,
        "sysroot": sysroot,
        "packageManager": package_manager,
        "packageCacheDir": str(cache),
    }
    peer = Peer(server, package_manager, sysroot, target, cache)
    try:
        peer.request(
            "initialize",
            {
                "processId": None,
                "capabilities": {},
                "workspaceFolders": [{"name": "mixed", "uri": root.as_uri()}],
                "initializationOptions": options,
            },
        )
        peer.notify("initialized", {})
        symbols = peer.request("workspace/symbol", {"query": "testScenario"})
        assert [item["location"]["uri"] for item in symbols] == [source.as_uri()], symbols
        assert not peer.diagnostics, peer.diagnostics

        params = {
            "textDocument": {"uri": source.as_uri()},
            "position": {"line": 2, "character": text.splitlines()[2].index("answer")},
        }
        definition = peer.request("textDocument/definition", params)
        assert definition["uri"] == support.as_uri(), definition

        peer.notify(
            "workspace/didChangeConfiguration",
            {"settings": {"scoop": {**options, "purpose": "production"}}},
        )
        assert peer.request("textDocument/definition", params) is None
        error = peer.diagnostics[(root / "checks/Cone.toml").as_uri()]["diagnostics"][0]
        assert error["code"] == "E2111" and "no locked resolution" in error["message"], error

        peer.notify("workspace/didChangeConfiguration", {"settings": {"scoop": options}})
        assert peer.request("textDocument/definition", params) == definition
        assert all(not entry["diagnostics"] for entry in peer.diagnostics.values()), (
            peer.diagnostics
        )
        assert (lock.read_bytes(), lock.stat().st_mtime_ns) == before
        peer.finish()
    finally:
        if peer.process.poll() is None:
            peer.process.kill()
            peer.process.wait(timeout=10)


def main():
    server, package_manager, directory, sysroot, target = sys.argv[1:]
    work = Path(directory)
    root = work / "workspace"
    repository = work / "repository"
    write(repository, "Cone.toml", manifest("shared"))
    library = write(
        repository, "src/value.scoop", "package shared\npublic fun selected(): Boolean = true\n"
    )
    run(["git", "init", "--quiet", str(repository)])
    run(["git", "-C", str(repository), "add", "."])
    run(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "original",
        ]
    )
    # The root Cone is intentionally excluded from the build default selection.
    write(
        root,
        "Cone.toml",
        manifest("root")
        + '[workspace]\nmembers = ["other", "missing"]\ndefault-members = ["other"]\n'
        + f'[dependencies]\n"test:shared" = {{git = "{repository.as_uri()}"}}\n',
    )
    write(work, "path/Cone.toml", manifest("shared", "2.0.0"))
    write(work, "path/src/value.scoop", 'package shared\npublic fun selected(): String = "two"\n')
    write(
        root,
        "other/Cone.toml",
        manifest("other") + '[dependencies]\n"test:shared" = {path = "../../path"}\n',
    )
    write(
        root,
        "missing/Cone.toml",
        manifest("missing") + '[dependencies]\n"test:absent" = {path = "../../absent"}\n',
    )
    source_text = "import shared.selected\nfun use() { val item = selected(); item. }\n"
    source_paths = [write(root, f"{prefix}src/use.scoop", source_text) for prefix in ("", "other/")]
    missing = write(root, "missing/src/local.scoop", "fun localOnly(): Boolean = true\n")
    cache = work / "packages"
    lock = root / "Cone.lock"
    assert not lock.exists()

    def initialize(peer):
        peer.request(
            "initialize",
            {
                "processId": None,
                "workspaceFolders": [{"name": "workspace", "uri": root.as_uri()}],
                "initializationOptions": {
                    "target": target,
                    "sysroot": sysroot,
                    "packageManager": package_manager,
                    "packageCacheDir": str(cache),
                },
                "capabilities": {
                    "textDocument": {"documentSymbol": {"hierarchicalDocumentSymbolSupport": True}}
                },
            },
        )
        peer.notify("initialized", {})

    peer = Peer(server, package_manager, sysroot, target, cache)
    try:
        initialize(peer)
        symbols = peer.request("workspace/symbol", {"query": "localOnly"})
        assert any(item["name"] == "localOnly" for item in symbols), symbols
        diagnostics = peer.diagnostics[(root / "missing/Cone.toml").as_uri()]["diagnostics"]
        assert any("absent" in item["message"] for item in diagnostics), diagnostics
        assert lock.exists()
        assert not (root / "other/Cone.lock").exists()
        assert not (root / "target").exists()
        peer.finish()
    finally:
        if peer.process.poll() is None:
            peer.process.kill()
            peer.process.wait(timeout=10)
    before = (lock.read_bytes(), lock.stat().st_mtime_ns)
    # Moving the branch must not change what the editor sees from the locked checkout.
    library.write_text('package shared\npublic fun selected(): String = "new branch"\n')
    run(["git", "-C", str(repository), "add", "."])
    run(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "advance",
        ]
    )
    peer = Peer(server, package_manager, sysroot, target, cache)
    try:
        initialize(peer)
        for path in [*source_paths, missing]:
            peer.notify(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": path.as_uri(),
                        "version": 1,
                        "languageId": "scoop",
                        "text": path.read_text(),
                    }
                },
            )
        for path, expected in zip(source_paths, ("Boolean", "String"), strict=True):
            hover = peer.request(
                "textDocument/hover",
                {
                    "textDocument": {"uri": path.as_uri()},
                    "position": {
                        "line": 1,
                        "character": source_text.splitlines()[1].rindex("item."),
                    },
                },
            )
            assert hover["contents"]["value"] == f"item: {expected}", (
                path,
                hover,
                peer.diagnostics,
            )
        symbols = peer.request(
            "textDocument/documentSymbol", {"textDocument": {"uri": missing.as_uri()}}
        )
        assert symbols[0]["name"] == "localOnly", symbols
        symbols = peer.request("workspace/symbol", {"query": "localOnly"})
        assert any(item["name"] == "localOnly" for item in symbols), symbols
        diagnostics = peer.diagnostics[(root / "missing/Cone.toml").as_uri()]["diagnostics"]
        error = next(item for item in diagnostics if item["code"] == "E2111")
        assert "no locked resolution" in error["message"], error
        assert any(
            note["code"] == "H2101"
            and note["kind"] == "help"
            and note["message"] == "run scoop pm resolve"
            for note in error["data"]["notes"]
        ), error
        assert (
            peer.request(
                "textDocument/hover",
                {
                    "textDocument": {"uri": missing.as_uri()},
                    "position": {"line": 0, "character": 5},
                },
            )
            is None
        )
        assert (lock.read_bytes(), lock.stat().st_mtime_ns) == before
        assert not (root / "target").exists()
        peer.finish()
    finally:
        if peer.process.poll() is None:
            peer.process.kill()
            peer.process.wait(timeout=10)
    check_mixed_purposes(
        server, package_manager, (work / "mixed").resolve(), sysroot, target, cache
    )
    print("locked workspace inputs passed")


if __name__ == "__main__":
    main()
