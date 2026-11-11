"""Prepare diagnostic copies for TCP echo layers without changing production libraries."""

import argparse
import shutil
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    output = args.output.resolve()
    templates = repo / "tests/benchmarks/tcp-echo/layers"
    cone = output / "sysroot/src/scoop.asyncio"
    cone.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(
        repo / "sysroot/src/scoop.asyncio",
        cone,
        ignore=shutil.ignore_patterns("target", "__pycache__", ".ruff_cache"),
    )
    (cone.parent / "scoop.platform").symlink_to(repo / "sysroot/src/scoop.platform")
    (output / "sysroot/vendor").symlink_to(repo / "sysroot/vendor")
    for name in ["raw", "callback"]:
        shutil.copy2(templates / f"{name}.scoop", cone / f"src/diagnostic_{name}.scoop")
    path = cone / "src/tcp_transfer.scoop"
    source = path.read_text()
    start = source.index("private fun prepareTcpTransfer(")
    end = source.index("\nprivate class TcpTransfer(", start)
    preparation = source[start:end]
    # The completion runs on the reactor, so retain the connection's actual Job
    # instead of asking for an ambient coroutine Context on every callback.
    preparation = preparation.replace(
        "private fun prepareTcpTransfer(", "internal fun diagnosticPrepareTransfer(", 1
    )
    preparation = preparation.replace(
        "    writing: Boolean\n", "    writing: Boolean,\n    job: Job\n", 1
    )
    preparation = preparation.replace("): TcpTransfer {", "): SuspendRegistration<Long> {", 1)
    preparation = preparation.replace(
        "    val job = owner.loop.requireJob()", "    job.ensureActive()", 1
    )
    path.write_text(source + "\n" + preparation)
    server = (repo / "tests/benchmarks/tcp-echo/server.scoop").read_text()
    start = server.index("private suspend fun echo(")
    end = server.index("private suspend fun serve(", start)
    callback = (
        server[:start]
        + """private suspend fun echo(stream: AsyncTcpStream, bufferSize: Long) {
    diagnosticCallbackEcho(stream, bufferSize)
}

"""
        + server[end:]
    )
    (output / "callback_server.scoop").write_text(callback)
    (output / "raw_server.scoop").write_text("""//! dep: scoop:scoop.asyncio:0.1.0

import scoop.asyncio.*
import scoop.text.*

@Extern(lib = "echo_metrics", name = "echo_metrics")
private fun snapshot()

fun main(args: Array<String>) {
    @Unsafe { snapshot() }
    diagnosticRawEcho(args[1L].toInt(), args[2L].toInt(), args[3L].toInt(), args[4L].toLong())
    @Unsafe { snapshot() }
}
""")
    shutil.copy2(templates / "native.c", output / "native.c")
    print(output)


if __name__ == "__main__":
    main()
