"""Copy asyncio sources and insert allocation-only probes for one echo worker."""

import argparse
import shutil
from pathlib import Path


def replace(path, old, new):
    source = path.read_text()
    if source.count(old) != 1:
        raise ValueError(f"expected one instrumentation site in {path}: {old!r}")
    path.write_text(source.replace(old, new))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    output = args.output.resolve()
    cone = output / "sysroot/src/scoop.asyncio"
    cone.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(repo / "sysroot/src/scoop.asyncio", cone)
    (cone.parent / "scoop.platform").symlink_to(repo / "sysroot/src/scoop.platform")
    (output / "sysroot/vendor").symlink_to(repo / "sysroot/vendor")
    source = cone / "src"
    (source / "allocation_probe.scoop").write_text(
        """package scoop.asyncio

@Extern(lib = "echo_alloc", name = "echo_alloc_enter")
private fun allocationEnter(phase: Int)

@Extern(lib = "echo_alloc", name = "echo_alloc_leave")
private fun allocationLeave()

internal fun traceEnter(phase: Int) {
    @Unsafe { allocationEnter(phase) }
}

internal fun traceLeave() {
    @Unsafe { allocationLeave() }
}
"""
    )
    path = source / "loop_owner.scoop"
    replace(
        path,
        "    fun requireJob(cancellable: Boolean = true): Job {",
        "    fun requireJob(cancellable: Boolean = true): Job {\n        traceEnter(1)",
    )
    replace(path, "        return job", "        traceLeave()\n        return job")
    path = source / "byte_transfer.scoop"
    replace(
        path, "        acquire()", "        traceEnter(2)\n        acquire()\n        traceLeave()"
    )
    replace(
        path,
        "            borrow = Some(borrowTransfer(buffer, count, writing))",
        "            traceEnter(3)\n            borrow = Some(borrowTransfer(buffer, count, writing))\n            traceLeave()",
    )
    replace(
        path,
        "            bind(current)",
        "            traceEnter(5)\n            bind(current)\n            traceLeave()\n            traceEnter(4)",
    )
    replace(path, "            return this", "            traceLeave()\n            return this")
    replace(
        path,
        "        loop.submit(this)",
        "        traceEnter(6)\n        loop.submit(this)\n        traceLeave()",
    )
    replace(
        path,
        "        target.resume(result)",
        "        traceEnter(10)\n        target.resume(result)\n        traceLeave()",
    )
    replace(
        path,
        "        val target = continuation!!",
        "        traceEnter(9)\n        val target = continuation!!",
    )
    replace(
        path,
        "                        release()",
        "                        release()\n                        traceLeave()",
    )
    path = source / "loop_owner.scoop"
    replace(
        path,
        "    fun beginOperation(operation: LoopOperation) {",
        "    fun beginOperation(operation: LoopOperation) {\n        traceEnter(7)",
    )
    replace(
        path,
        "        dispatchBlocking(pointer!!)\n    }",
        "        dispatchBlocking(pointer!!)\n        traceLeave()\n    }",
    )
    replace(
        path,
        "    fun complete(event: NativeEvent) {",
        "    fun complete(event: NativeEvent) {\n        traceEnter(8)",
    )
    replace(
        path,
        "        operation.complete(event)",
        "        operation.complete(event)\n        traceLeave()",
    )
    path = source / "loop_drive.scoop"
    replace(path, "        pumping = true", "        pumping = true\n        traceEnter(11)")
    replace(
        path,
        "            pumpAgain = false",
        "            pumpAgain = false\n            traceLeave()",
    )

    server = output / "server.scoop"
    shutil.copy2(repo / "tests/benchmarks/tcp-echo/server.scoop", server)
    replace(
        server,
        "private suspend fun echo(",
        """@Extern(lib = "echo_alloc", name = "echo_alloc_start")
private fun allocationStart()
@Extern(lib = "echo_alloc", name = "echo_alloc_end")
private fun allocationEnd()
@Extern(lib = "echo_alloc", name = "echo_alloc_report")
private fun allocationReport()
@Extern(lib = "echo_alloc", name = "echo_alloc_enter")
private fun allocationEnter(phase: Int)
@Extern(lib = "echo_alloc", name = "echo_alloc_leave")
private fun allocationLeave()

private suspend fun echo(""",
    )
    replace(
        server,
        "                buffer.clear()",
        "                @Unsafe { allocationStart() }\n                buffer.clear()",
    )
    replace(
        server,
        "                val count = peer.readInto(buffer)",
        "                @Unsafe { allocationEnter(12) }\n                val count = peer.readInto(buffer)\n                @Unsafe { allocationLeave() }",
    )
    replace(
        server,
        "                peer.writeFully(buffer, count)",
        "                @Unsafe { allocationEnter(13) }\n                peer.writeFully(buffer, count)\n                @Unsafe { allocationLeave(); allocationEnd() }",
    )
    replace(
        server,
        "    val bufferSize = args[4L].toLong()",
        '    val bufferSize = args[4L].toLong()\n    if (workerCount != 1 || connections != 1)\n        throw IllegalArgumentException(Some("allocation probe requires one worker and one connection"))',
    )
    replace(
        server,
        "        snapshot()\n    }\n}",
        "        snapshot()\n        allocationReport()\n    }\n}",
    )
    print(cone)
    print(server)


if __name__ == "__main__":
    main()
