"""External process checks for the published examples."""

import contextlib
import json
import os
import signal
import socket
import subprocess
import time


def run(command, cwd, environment, log, expected_code=0):
    result = subprocess.run(
        command, cwd=cwd, env=environment, capture_output=True, text=True, timeout=180
    )
    log.write_text(json.dumps(command, ensure_ascii=False) + "\n" + result.stdout + result.stderr)
    if result.returncode != expected_code:
        raise AssertionError(f"exit {result.returncode}, expected {expected_code}; see {log}")
    return result


def build(command, source, name, cwd, environment, logs):
    executable = cwd / (name + (".exe" if os.name == "nt" else ""))
    run(
        command + ["build", "--release", source, "-o", str(executable)],
        cwd,
        environment,
        logs / f"{name}-build.log",
    )
    return executable


def http_client(command, cwd, environment, logs):
    server_exe = build(command, "server.scoop", "http-server", cwd, environment, logs)
    client = build(command, "client.scoop", "http-client", cwd, environment, logs)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 8080))
    server_log = logs / "server.log"
    with server_log.open("w") as output:
        server = subprocess.Popen(
            [str(server_exe)], cwd=cwd, env=environment, stdout=output, stderr=subprocess.STDOUT
        )
        try:
            deadline = time.monotonic() + 30
            while True:
                if server.poll() is not None:
                    raise AssertionError(f"server exited; see {server_log}")
                try:
                    with socket.create_connection(("127.0.0.1", 8080), timeout=0.2):
                        break
                except OSError:
                    pass
                if time.monotonic() > deadline:
                    raise AssertionError(f"server did not start; see {server_log}")
                time.sleep(0.05)
            result = run(
                [str(client), "http://127.0.0.1:8080/"], cwd, environment, logs / "client.log"
            )
            assert not result.stderr
            missing = run(
                [str(client), "http://127.0.0.1:8080/missing"],
                cwd,
                environment,
                logs / "missing-route.log",
                1,
            )
            assert not missing.stdout and missing.stderr
            # An owned, bound but non-listening port gives a local connection failure.
            with socket.socket() as closed:
                closed.bind(("127.0.0.1", 0))
                port = closed.getsockname()[1]
                failed = run(
                    [str(client), f"http://127.0.0.1:{port}/"],
                    cwd,
                    environment,
                    logs / "connection-failure.log",
                    1,
                )
            assert not failed.stdout and failed.stderr
            return result
        finally:
            if server.poll() is None:
                server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)


def processes(command, cwd, environment, logs):
    child = build(command, "child.scoop", "process-child", cwd, environment, logs)
    parent = build(command, "main.scoop", "process-parent", cwd, environment, logs)
    result = run([str(parent), str(child)], cwd, environment, logs / "run.log")
    assert not result.stderr
    return result.stdout


def environment_variables(command, cwd, environment, logs):
    executable = build(command, "main.scoop", "environment", cwd, environment, logs)
    for label, value in (("missing", None), ("empty", ""), ("present", "Ada")):
        env = environment.copy()
        env.pop("SCOOP_EXAMPLE_LABEL", None)
        if value is not None:
            env["SCOOP_EXAMPLE_LABEL"] = value
        result = run([str(executable)], cwd, env, logs / f"{label}.log")
        expected = (
            "missing\neffective: 'guest'\n"
            if value is None
            else f"present: '{value}'\neffective: '{value}'\n"
        )
        assert result.stdout == expected and not result.stderr


def file_report(command, cwd, environment, logs):
    run(command + ["test", "--release", "."], cwd, environment, logs / "test.log")
    executable = build(command, ".", "file-report", cwd, environment, logs)
    sample = run([str(executable), "sample.txt"], cwd, environment, logs / "sample.log")
    assert not sample.stderr
    empty = cwd / "temporary" / "empty.txt"
    invalid = cwd / "temporary" / "invalid.txt"
    empty.write_bytes(b"")
    invalid.write_bytes(b"\xff")
    try:
        result = run([str(executable), str(empty)], cwd, environment, logs / "empty.log")
        assert result.stdout == "lines=0, scalars=0, utf8Bytes=0\n" and not result.stderr
        for label, arguments, code in (
            ("missing", ["does-not-exist.txt"], 1),
            ("invalid-utf8", [str(invalid)], 1),
            ("usage", [], 2),
        ):
            failed = run(
                [str(executable), *arguments], cwd, environment, logs / f"{label}.log", code
            )
            assert not failed.stdout and failed.stderr
    finally:
        empty.unlink()
        invalid.unlink()
    return sample.stdout


def pty_output(executable, cwd, environment, columns):
    # Standard-library POSIX PTY: the child gets a controlling terminal and a known size.
    import errno
    import fcntl
    import pty
    import select
    import struct
    import termios

    pid, master = pty.fork()
    if pid == 0:
        try:
            os.chdir(cwd)
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 8, columns, 0, 0))
            os.execve(executable, [str(executable)], environment)
        finally:
            os._exit(127)
    output = bytearray()
    status = None
    try:
        deadline = time.monotonic() + 10
        while True:
            if time.monotonic() >= deadline:
                raise AssertionError("terminal example did not finish")
            ready, _, _ = select.select([master], [], [], 0.1)
            if ready:
                try:
                    chunk = os.read(master, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    chunk = b""
                if not chunk:
                    break
                output.extend(chunk)
        while status is None:
            waited, code = os.waitpid(pid, os.WNOHANG)
            if waited:
                status = code
            elif time.monotonic() >= deadline:
                raise AssertionError("terminal example closed output without exiting")
            else:
                time.sleep(0.01)
        assert os.waitstatus_to_exitcode(status) == 0
        return output.decode().replace("\r\n", "\n")
    finally:
        if status is None:
            with contextlib.suppress(ProcessLookupError):
                os.killpg(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
        os.close(master)


def terminal_output(command, cwd, environment, logs):
    executable = build(command, "main.scoop", "terminal-output", cwd, environment, logs)
    redirected = run([str(executable)], cwd, environment, logs / "redirected.log")
    assert not redirected.stderr and "\x1b" not in redirected.stdout
    if os.name == "nt":
        raise AssertionError(
            "the example PTY checks need a POSIX host; Windows interaction remains unverified"
        )
    for label, columns, no_color in (
        ("wide", 80, False),
        ("narrow", 12, False),
        ("plain", 80, True),
    ):
        env = {**environment, "TERM": "xterm-256color"}
        env.pop("NO_COLOR", None)
        if no_color:
            env["NO_COLOR"] = ""
        output = pty_output(executable, cwd, env, columns)
        (logs / f"{label}.log").write_text(output)
        assert ("\x1b" not in output) if no_color else ("\x1b" in output)
        assert "bytes=18, scalars=7, graphemes=4" in output
        assert (
            ("Scoop text report" in output)
            if columns >= 24
            else ("Scoop text report" not in output)
        )
    return redirected.stdout
