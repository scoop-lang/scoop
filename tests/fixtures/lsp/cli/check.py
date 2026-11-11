"""CLI queries consume the selected real toolchain and agree with stdio LSP."""

import json
import subprocess
import sys
from pathlib import Path


def main():
    server, pm, root, sysroot, target = sys.argv[1:]
    root = Path(root).resolve()
    source = root / "src/value.scoop"
    text = source.read_text()
    use_line = next(i for i, line in enumerate(text.splitlines()) if line.startswith("fun use"))
    identity_line = next(
        i for i, line in enumerate(text.splitlines()) if line.startswith("fun identity")
    )
    # Exercise LSP newlines independently of the checkout's LF policy.
    source.write_bytes(text.replace("\n", "\r\n").encode())
    options = {
        "sysroot": sysroot,
        "target": target,
        "packageManager": pm,
        "packageCacheDir": str(root.parent / "packages"),
    }
    base = [
        server,
        "query",
        "--project",
        str(root),
        "--sysroot",
        sysroot,
        "--target",
        target,
        "--package-manager",
        pm,
        "--package-cache-dir",
        options["packageCacheDir"],
    ]

    def query(kind, *args, buffer=None):
        command = [*base, kind, *args, "--format", "json"]
        if buffer is not None:
            command.append("--stdin")
        run = subprocess.run(
            command, input=buffer, capture_output=True, text=True, cwd=root, timeout=60
        )
        assert run.returncode == 0, run.stderr
        assert not run.stderr, run.stderr
        value = json.loads(run.stdout)
        assert (value["schema"], value["kind"], value["rangeBase"]) == (1, "query", 0), value
        return value

    def at(kind, line, token, *, encoding="utf-32", buffer=None, after=False):
        lines = (buffer if buffer is not None else text).splitlines()
        index = lines[line].index(token) + (len(token) if after else 0)
        prefix = lines[line][:index]
        column = {
            "utf-32": len(prefix),
            "utf-16": len(prefix.encode("utf-16-le")) // 2,
            "utf-8": len(prefix.encode()),
        }[encoding]
        args = [
            "src/value.scoop",
            "--line",
            str(line + 1),
            "--column",
            str(column + 1),
            "--position-encoding",
            encoding,
        ]
        return query(kind, *args, buffer=buffer), {"line": line, "character": column}

    for encoding in ("utf-8", "utf-16", "utf-32"):
        hover, position = at("hover", use_line, "value()", encoding=encoding)
        assert hover["status"] == "ok", hover
        assert hover["positionEncoding"] == encoding
        assert "fun value(): Int" in json.dumps(hover["result"]), hover
        expected = stdio_hover(server, root, source, options, encoding, position)
        assert hover["result"] == expected, (hover, expected)

    definition, _ = at("definition", use_line, "identity")
    assert definition["result"]["uri"] == source.as_uri(), definition
    assert definition["result"]["range"]["start"]["line"] == identity_line
    references, _ = at("references", identity_line, "identity")
    assert len(references["result"]) == 2, references
    type_definition, _ = at("type-definition", use_line, "`😀名`")
    assert type_definition["result"]["uri"] == source.as_uri(), type_definition
    implementations, _ = at("implementation", 2, "Valued")
    assert implementations["result"], implementations

    signature, _ = at(
        "signature",
        2,
        "identity(",
        after=True,
        buffer="package test.editor\nfun identity(number: Int): Int = number\nfun call(): Int = identity( \n",
    )
    assert signature["status"] == "partial", signature
    assert "identity" in json.dumps(signature["result"]), signature
    completion, _ = at(
        "completion",
        2,
        "box.",
        after=True,
        buffer="package test.editor\nclass Box { fun value(): Int = 42 }\nfun call(box: Box) { box. }\n",
    )
    assert any(item["label"] == "value" for item in completion["result"]["items"]), completion
    extension_source = 'package test.editor\nimport scoop.getOrElse\nfun fallback(value: String?): String = value.getOrElse { "none" }\n'
    extension, _ = at("definition", 2, "getOrElse", buffer=extension_source)
    assert extension["status"] == "ok", extension
    assert extension["result"]["uri"].endswith("scoop.std/src/results/option.scoop"), extension
    extension_completion, _ = at(
        "completion",
        2,
        "value.",
        after=True,
        buffer="package test.editor\nimport scoop.getOrElse\nfun fallback(value: String?) { value. }\n",
    )
    assert any(item["label"] == "getOrElse" for item in extension_completion["result"]["items"]), (
        extension_completion
    )
    nested, _ = at(
        "signature",
        2,
        "passthrough(",
        after=True,
        buffer="package test.editor\nfun <T> passthrough(value: T): T = value\nfun nested(value: Array<String?>): Array<String?> = passthrough(value)\n",
    )
    assert nested["status"] == "ok", nested
    label = nested["result"]["signatures"][0]["label"]
    assert "Array" in label and "String" in label and ("Option" in label or "?" in label), nested
    assert query("diagnostics", "src/value.scoop")["status"] == "empty"
    assert query("workspace-symbols", "missingName")["status"] == "empty"
    assert query("workspace-symbols", "CliTests")["status"] == "empty"
    assert (
        query("workspace-symbols", "CliTests", "--purpose", "test")["result"][0]["name"]
        == "CliTests"
    )
    tests = query("diagnostics", "tests/local.scoop", "--purpose", "test")
    # A reused fixture directory may already have its test resolution.
    if tests["status"] == "partial":
        assert "no locked resolution" in json.dumps(tests), tests
    else:
        assert tests["status"] == "empty", tests
    resolved = subprocess.run(
        [
            pm,
            "resolve",
            str(root),
            "--purpose",
            "test",
            "--offline",
            "--sysroot",
            sysroot,
            "--target",
            target,
            "--package-cache-dir",
            options["packageCacheDir"],
        ],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert resolved.returncode == 0, resolved.stderr
    tests = query("diagnostics", "tests/local.scoop", "--purpose", "test")
    assert tests["status"] == "empty", tests

    run = subprocess.run(
        [*base, "hover", "src/value.scoop", "--line", str(identity_line + 1), "--column", "6"],
        capture_output=True,
        text=True,
        cwd=root,
        timeout=60,
    )
    assert run.returncode == 0 and not run.stderr, run
    assert "fun identity(number: Int): Int" in run.stdout, run.stdout
    assert not (root / "target").exists()
    print("language service CLI queries passed")


def stdio_hover(server, root, source, options, encoding, position):
    # Small sequential peer: the production CLI uses lsp-server framing instead.
    with subprocess.Popen(
        [server, "--stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    ) as process:

        def send(method, params, request_id=None):
            message = {"jsonrpc": "2.0", "method": method, "params": params}
            if request_id is not None:
                message["id"] = request_id
            data = json.dumps(message).encode()
            process.stdin.write(f"Content-Length: {len(data)}\r\n\r\n".encode() + data)
            process.stdin.flush()

        def response(request_id):
            while True:
                headers = {}
                while (line := process.stdout.readline()) not in (b"\r\n", b"\n"):
                    assert line, "LSP closed before response"
                    key, value = line.decode().split(":", 1)
                    headers[key.lower()] = value.strip()
                message = json.loads(process.stdout.read(int(headers["content-length"])))
                if message.get("id") == request_id:
                    assert "error" not in message, message
                    return message["result"]

        try:
            send(
                "initialize",
                {
                    "capabilities": {"general": {"positionEncodings": [encoding]}},
                    "workspaceFolders": [{"uri": root.as_uri(), "name": "app"}],
                    "initializationOptions": options,
                },
                1,
            )
            response(1)
            send("initialized", {})
            send(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": source.as_uri(),
                        "languageId": "scoop",
                        "version": 1,
                        "text": source.read_bytes().decode(),
                    }
                },
            )
            send(
                "textDocument/hover",
                {"textDocument": {"uri": source.as_uri()}, "position": position},
                2,
            )
            result = response(2)
            send("shutdown", None, 3)
            response(3)
            send("exit", None)
            assert process.wait(timeout=5) == 0
            assert not process.stderr.read()
            return result
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
