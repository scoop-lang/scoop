"""Real compiler diagnostics and versioned LSP edits share the same causes."""

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import url2pathname

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "workspace"))
from diagnostics import published  # noqa: E402
from peer import Peer  # noqa: E402


def point(text, byte_offset):
    prefix = text.encode()[:byte_offset].decode()
    lines = prefix.split("\n")
    return {"line": len(lines) - 1, "character": len(lines[-1].rstrip("\r"))}


def apply(action, text, uri, version):
    changes = action["edit"]["documentChanges"]
    assert len(changes) == 1, changes
    change = changes[0]
    assert change["textDocument"] == {"uri": uri, "version": version}, change
    assert len(change["edits"]) == 1, change
    edit = change["edits"][0]
    assert edit["range"]["start"] == edit["range"]["end"], edit
    position = edit["range"]["start"]
    lines = text.splitlines(keepends=True)
    offset = sum(map(len, lines[: position["line"]])) + position["character"]
    return text[:offset] + edit["newText"] + text[offset:]


class Checks:
    def __init__(self, scoop, compiler, server, pm, root, sysroot, target, cache):
        self.root = Path(root).resolve()
        self.source = self.root / "src/case.scoop"
        self.uri = self.source.as_uri()
        self.version = 0
        self.compile_command = [
            scoop,
            "--package-manager",
            pm,
            "check",
            str(self.root),
            "--scoopc",
            compiler,
            "--release",
            "--sysroot",
            sysroot,
            "--target",
            target,
            "--cache-dir",
            cache,
            "--package-cache",
            str(self.root.parent / "packages"),
        ]
        self.query_command = [
            server,
            "query",
            "--project",
            str(self.root),
            "--sysroot",
            sysroot,
            "--target",
            target,
            "--package-manager",
            pm,
            "--package-cache-dir",
            str(self.root.parent / "packages"),
            "--position-encoding",
            "utf-32",
        ]
        self.peer = Peer(server, pm, sysroot, target, self.root.parent / "packages")
        self.peer.resolve(self.root)
        self.peer.request(
            "initialize",
            {
                "processId": None,
                "workspaceFolders": [{"name": "diagnostics", "uri": self.root.as_uri()}],
                "initializationOptions": {
                    "target": target,
                    "sysroot": sysroot,
                    "packageManager": pm,
                    "packageCacheDir": str(self.peer.cache),
                },
                "capabilities": {
                    "general": {"positionEncodings": ["utf-32"]},
                    "workspace": {"workspaceEdit": {"documentChanges": True}},
                    "textDocument": {
                        "codeAction": {
                            "codeActionLiteralSupport": {
                                "codeActionKind": {"valueSet": ["quickfix"]}
                            },
                            "dataSupport": True,
                            "resolveSupport": {"properties": ["edit"]},
                        }
                    },
                },
            },
        )
        self.peer.notify("initialized", {})

    def set_source(self, text):
        self.text = text
        self.source.write_bytes(text.encode())
        self.version += 1
        document = {"uri": self.uri, "version": self.version}
        if self.version == 1:
            self.peer.notify(
                "textDocument/didOpen",
                {
                    "textDocument": {**document, "languageId": "scoop", "text": text},
                },
            )
        else:
            self.peer.notify(
                "textDocument/didChange",
                {
                    "textDocument": document,
                    "contentChanges": [{"text": text}],
                },
            )

    def compiler(self, human=False):
        run = subprocess.run(
            [*self.compile_command, "--message-format", "human" if human else "json"],
            cwd=self.root,
            capture_output=True,
            text=True,
            timeout=90,
        )
        assert run.stdout == "", run.stdout
        if human:
            return run
        events = [json.loads(line) for line in run.stderr.splitlines()]
        errors = [item for item in events if item["kind"] == "diagnostic"]
        assert run.returncode == (1 if errors else 0), (run.returncode, events)
        return errors

    def agree(self, text, editor_extra=()):
        self.set_source(text)
        cli = self.compiler()
        lsp = published(self.peer, self.uri, self.version)
        # Editing continues past a broken import header. The strict compiler
        # stops at that header, so only this named dependent error is additional.
        comparable = [item for item in lsp if item["message"] not in editor_extra]
        assert sorted(item["message"] for item in lsp if item["message"] in editor_extra) == sorted(
            editor_extra
        ), lsp

        def key(item):
            return (
                item["message"],
                item["range"]["start"]["line"],
                item["range"]["start"]["character"],
            )

        converted = [
            {
                "message": item["message"],
                "range": {
                    "start": point(text, item["origin"]["start"]),
                    "end": point(text, item["origin"]["end"]),
                },
                "notes": item["notes"],
            }
            for item in cli
        ]
        assert len(converted) == len(comparable), (cli, lsp)
        for expected, actual in zip(
            sorted(converted, key=key), sorted(comparable, key=key), strict=True
        ):
            assert expected["message"] == actual["message"], (expected, actual)
            assert expected["range"] == actual["range"], (expected, actual)
            paths = sorted(note["importPath"] for note in expected["notes"] if "importPath" in note)
            assert paths == actual.get("data", {}).get("imports", []), (expected, actual)
            assert sorted(note["message"] for note in expected["notes"]) == sorted(
                note["message"] for note in actual.get("relatedInformation", [])
            ), (expected, actual)
            for note in expected["notes"]:
                related = next(
                    item
                    for item in actual["relatedInformation"]
                    if item["message"] == note["message"]
                )
                location = related["location"]
                path = Path(url2pathname(urlsplit(location["uri"]).path))
                origin = note["origin"]
                assert path.as_posix().endswith("/" + origin["path"]), (note, related)
                contents = path.read_bytes().decode()
                assert location["range"] == {
                    "start": point(contents, origin["start"]),
                    "end": point(contents, origin["end"]),
                }, (note, related)
        return cli, lsp

    def actions(self, diagnostic):
        return self.peer.request(
            "textDocument/codeAction",
            {
                "textDocument": {"uri": self.uri},
                "range": diagnostic["range"],
                "context": {"diagnostics": [], "only": ["quickfix"]},
            },
        )

    def import_case(self, text, paths):
        cli, lsp = self.agree(text)
        assert len(cli) == 1, cli
        assert sorted(
            note["importPath"] for note in cli[0]["notes"] if "importPath" in note
        ) == sorted(paths), cli
        actions = self.actions(lsp[0])
        assert len(actions) == len(paths), actions
        fixes = [apply(action, text, self.uri, self.version) for action in actions]
        for fixed in fixes:
            assert self.agree(fixed) == ([], []), fixed
        return actions


def check_roots(checks):
    header = "package test.app\nimport scoop.test.assertEquals\nfun optional(value: String?): String? = value\n"
    for call in [
        'assertEquals(Some("x"), optional("bad"))',
        'assertEquals(optional("bad"), Some("x"))',
        'assertEquals(Some(Some("x")), Some(optional("bad")))',
    ]:
        cli, lsp = checks.agree(header + f"fun check() {{ {call} }}\n")
        assert len(cli) == 1, cli
        error = cli[0]
        assert "optional" in error["message"] and "Option<String>" in error["message"], error
        assert "assertEquals" not in error["message"] and "Equals" not in error["message"], error
        assert (
            checks.text.encode()[error["origin"]["start"] : error["origin"]["end"]] == b'"bad"'
        ), error
        assert checks.actions(lsp[0]) == []
        human = checks.compiler(human=True)
        assert human.returncode == 1, human.stderr
        assert all(line.strip() in human.stderr for line in error["message"].splitlines()), (
            human.stderr
        )
        assert checks.agree(checks.text.replace('optional("bad")', 'optional(Some("x"))')) == (
            [],
            [],
        )
    safe = (
        "package test.app\nimport scoop.test.assertEquals\nstruct Box(val value: String)\n"
        'fun boxed(value: String?): Box? = Some(Box("x"))\n'
        'fun check() { assertEquals(Some("x"), boxed("bad")?.value) }\n'
    )
    cli, _ = checks.agree(safe)
    assert len(cli) == 1 and "boxed" in cli[0]["message"], cli
    assert "assertEquals" not in cli[0]["message"], cli
    assert checks.agree(safe.replace('boxed("bad")', 'boxed(Some("x"))')) == ([], [])
    cli, _ = checks.agree(
        header + "fun <T : Equals<T>> nominal(value: T) {}\n"
        'fun nested() { assertEquals(Some("x"), optional("bad")) }\n'
        'fun unrelated() { nominal(Some("x")) }\n'
    )
    assert len(cli) == 2, cli
    assert any("nominal" in item["message"] and "Equals" in item["message"] for item in cli), cli
    cli, _ = checks.agree(
        "package test.app\nfun invalid(value: String?) { if (value != None) { value.charLength } }\n"
    )
    assert len(cli) == 1 and cli[0]["message"] == "type `Option<String>` has no fields", cli
    assert (
        checks.text.encode()[cli[0]["origin"]["start"] : cli[0]["origin"]["end"]] == b"charLength"
    ), cli


def check_imports(checks):
    for body, paths in [
        ("fun check() { choose() }", [["test", "first", "choose"], ["test", "second", "choose"]]),
        ('fun check() { "x".decorate() }', [["test", "first", "decorate"]]),
        ('fun check() { "x".localMark() }', [["test", "localtools", "localMark"]]),
        ("fun check(value: Record) {}", [["test", "first", "Record"]]),
        ("fun check(): Int = marker", [["test", "first", "marker"]]),
        ("fun check() { forwarded() }", [["test", "first", "forwarded"]]),
        (
            'fun check(value: String?): String = value.getOrElse { "empty" }',
            [["scoop", "getOrElse"]],
        ),
        (
            'fun check(value: String?): String? = value.orElse { Some("empty") }',
            [["scoop", "orElse"]],
        ),
        ("fun check() { `测 试`() }", [["test", "选 择", "测 试"]]),
        ("fun check() { 1.decorate() }", []),
        ("fun check() { secret() }", []),
        ("fun check() { hiddenCapability() }", []),
        ("fun check(value: Clash) {}", []),
        ("fun check() { 1.readable() }", []),
        (
            "struct Good() { public fun read(): Int = 1 }\nfun check(value: Good) { value.readable() }",
            [["test", "localtools", "readable"]],
        ),
    ]:
        checks.import_case(f"package test.app\n{body}\n", paths)
    # A new exact import cannot replace an existing binding with the same local name.
    checks.import_case(
        'package test.app\nimport test.first.marker as decorate\nfun check() { "x".decorate() }\n',
        [],
    )
    # Two exact imports still report real ambiguity, without inventing a third repair.
    cli, lsp = checks.agree(
        "package test.app\nimport test.first.choose\nimport test.second.choose\nfun check() { choose() }\n",
        editor_extra=("import binding for `choose` is unavailable",),
    )
    assert cli and any("ambiguous" in item["message"] for item in cli), cli
    assert all(checks.actions(item) == [] for item in lsp)


def check_versions_and_cli(checks):
    source = "package test.app\r\n// 中文😀\r\nfun check() { `测 试`() }\r\n"
    _, diagnostics = checks.agree(source)
    action = checks.actions(diagnostics[0])[0]
    resolved = checks.peer.request("codeAction/resolve", action)
    assert resolved["edit"] == action["edit"]
    checks.set_source(source + "// changed\n")
    request = checks.peer.send_request("codeAction/resolve", action)
    checks.peer.response(request, error_code=-32801)
    _, diagnostics = checks.agree(source)
    action = checks.actions(diagnostics[0])[0]
    provider = checks.root / "src/local.scoop"
    checks.peer.notify(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": provider.as_uri(),
                "version": 1,
                "languageId": "scoop",
                "text": provider.read_text() + "// another source changed\n",
            }
        },
    )
    request = checks.peer.send_request("codeAction/resolve", action)
    checks.peer.response(request, error_code=-32801)
    checks.peer.notify("textDocument/didClose", {"textDocument": {"uri": provider.as_uri()}})
    position = diagnostics[0]["range"]["start"]
    args = [
        "code-actions",
        str(checks.source),
        "--line",
        str(position["line"] + 1),
        "--column",
        str(position["character"] + 1),
    ]
    run = subprocess.run(
        [*checks.query_command, *args, "--format", "json"],
        capture_output=True,
        text=True,
        cwd=checks.root,
        timeout=30,
    )
    assert run.returncode == 0 and not run.stderr, (run.stdout, run.stderr)
    output = json.loads(run.stdout)
    assert output["status"] == "partial" and output["rangeBase"] == 0, output
    assert len(output["result"]) == 1, output
    action = output["result"][0]
    assert "data" not in action, action
    fixed = apply(action, source, checks.uri, 1)
    assert "import test.`选 择`.`测 试`\r\n" in fixed, fixed
    assert checks.agree(fixed) == ([], [])


def main():
    checks = Checks(*sys.argv[1:])
    try:
        check_roots(checks)
        check_imports(checks)
        check_versions_and_cli(checks)
        checks.peer.finish()
    finally:
        if checks.peer.process.poll() is None:
            checks.peer.process.kill()
            checks.peer.process.wait(timeout=10)
    print("diagnostic roots and import actions passed")


if __name__ == "__main__":
    main()
