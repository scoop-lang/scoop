"""Compare fixed Scoop and Go TCP echo binaries on separate Linux CPU sets."""

import argparse
import json
import os
import queue
import random
import statistics
import subprocess
import threading
import time
from pathlib import Path


def integers(value):
    return [int(part) for part in value.split(",")]


def process(pid):
    root = Path(f"/proc/{pid}")
    ticks = os.sysconf("SC_CLK_TCK")
    fields = (root / "stat").read_text().rsplit(")", 1)[1].split()
    result = {"user_s": int(fields[11]) / ticks, "system_s": int(fields[12]) / ticks}
    for line in (root / "status").read_text().splitlines():
        key, value = line.split(":", 1)
        if key in ("VmRSS", "VmHWM", "Threads"):
            result[key] = int(value.split()[0])
    result["context_switches"] = 0
    result["threads_cpu_s"] = {}
    for task in (root / "task").iterdir():
        try:
            row = (task / "stat").read_text().rsplit(")", 1)[1].split()
            result["threads_cpu_s"][task.name] = (int(row[11]) + int(row[12])) / ticks
            for line in (task / "status").read_text().splitlines():
                if line.startswith(("voluntary_ctxt_switches:", "nonvoluntary_ctxt_switches:")):
                    result["context_switches"] += int(line.split()[1])
        except FileNotFoundError:
            pass
    return result


def read_lines(stream, messages, output):
    try:
        for line in stream:
            output.write(line)
            output.flush()
            messages.put(line.strip())
    finally:
        messages.put(None)


def stop(child):
    if child is not None and child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()


def run_sample(args, configuration, connections, repeat):
    label, workers, cpu_count, binary = configuration
    directory = args.work / f"{label}-c{connections}-r{repeat}"
    saved = directory / "result.json"
    if saved.exists():
        result = json.loads(saved.read_text())
        assert result["status"] == "ok"
        return result
    directory.mkdir()
    server_cpus = args.server_cpus[:cpu_count]
    command = [
        "taskset",
        "-c",
        ",".join(map(str, server_cpus)),
        str(binary),
        str(workers),
        str(args.port),
        str(connections),
        str(args.buffer),
    ]
    client_command = [
        "taskset",
        "-c",
        ",".join(map(str, args.client_cpus)),
        str(args.client),
        "-address",
        f"127.0.0.1:{args.port}",
        "-connections",
        str(connections),
        "-payload",
        str(args.payload),
        "-seconds",
        str(args.seconds),
        "-warmup",
        str(args.warmup),
        "-workers",
        str(len(args.client_cpus)),
    ]
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("SCOOP_GC_")
        and key not in ("GOGC", "GOMEMLIMIT", "GODEBUG", "LD_PRELOAD")
    }
    result = {
        "configuration": label,
        "workers": workers,
        "connections": connections,
        "repeat": repeat,
        "server_command": command,
        "client_command": client_command,
        "server_cpus": server_cpus,
        "client_cpus": args.client_cpus,
        "payload_bytes": args.payload,
        "buffer_bytes": args.buffer,
        "warmup_s": args.warmup,
        "requested_seconds": args.seconds,
    }
    server = client = None
    reader = None
    with (
        (directory / "server.stdout").open("w") as server_output,
        (directory / "server.stderr").open("w") as server_error,
        (directory / "client.stdout").open("w") as client_output,
        (directory / "client.stderr").open("w") as client_error,
    ):
        try:
            server = subprocess.Popen(
                command, env=environment, stdout=server_output, stderr=server_error
            )
            deadline = time.monotonic() + 20
            while "READY\n" not in (directory / "server.stdout").read_text():
                if server.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f"server readiness failed: {directory}")
                time.sleep(0.02)
            client = subprocess.Popen(
                client_command,
                env=environment,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=client_error,
                text=True,
                bufsize=1,
            )
            messages = queue.Queue()
            reader = threading.Thread(
                target=read_lines, args=(client.stdout, messages, client_output)
            )
            reader.start()
            if messages.get(timeout=20) != "READY":
                raise RuntimeError("client did not establish all connections")
            client.stdin.write("RUN\n")
            client.stdin.flush()
            if messages.get(timeout=args.warmup + 20) != "MEASURE_START":
                raise RuntimeError("client did not start measurement")
            before = process(server.pid)
            client_before = process(client.pid)
            started = time.monotonic()
            samples = [before]
            deadline = started + args.seconds + 20
            while True:
                try:
                    message = messages.get(timeout=0.1)
                except queue.Empty:
                    if time.monotonic() > deadline:
                        raise TimeoutError("echo measurement") from None
                    samples.append(process(server.pid))
                    continue
                if message != "MEASURE_END":
                    raise RuntimeError(f"unexpected client event: {message}")
                break
            elapsed = time.monotonic() - started
            after = process(server.pid)
            client_after = process(client.pid)
            message = messages.get(timeout=10)
            if message is None or not message.startswith("RESULT "):
                raise RuntimeError("client result missing")
            load = json.loads(message[7:])
            assert load["requests"] > 0 and not load["errors"], load
            client.stdin.write("CLOSE\n")
            client.stdin.flush()
            client.stdin.close()
            assert client.wait(timeout=10) == 0
            reader.join(timeout=5)
            assert not reader.is_alive()
            assert server.wait(timeout=15) == 0
            snapshots = [
                json.loads(line[8:])
                for line in (directory / "server.stdout").read_text().splitlines()
                if line.startswith("METRICS ")
            ]
            assert len(snapshots) == 2, snapshots
            server_cpu = sum(after[key] - before[key] for key in ("user_s", "system_s"))
            client_cpu = sum(
                client_after[key] - client_before[key] for key in ("user_s", "system_s")
            )
            allocation = snapshots[1]["allocated_bytes"] - snapshots[0]["allocated_bytes"]
            result.update(
                **load,
                before=before,
                after=after,
                samples=samples,
                client_before=client_before,
                client_after=client_after,
                measured_wall_s=elapsed,
                server_cpu_pct=server_cpu / elapsed * 100,
                client_cpu_pct=client_cpu / elapsed * 100,
                cpu_us_per_request=server_cpu * 1e6 / load["requests"],
                system_cpu_pct=100
                * (after["system_s"] - before["system_s"])
                / max(server_cpu, 1e-9),
                rss_mib_median=statistics.median(s["VmRSS"] for s in samples) / 1024,
                rss_mib_peak=max(s["VmHWM"] for s in [*samples, after]) / 1024,
                threads_peak=max(s["Threads"] for s in [*samples, after]),
                context_switches_per_request=(
                    after["context_switches"] - before["context_switches"]
                )
                / load["requests"],
                allocated_kib_per_request_with_warmup=allocation
                / load["requests_with_warmup"]
                / 1024,
                gc_before=snapshots[0],
                gc_after=snapshots[1],
                status="ok",
            )
        finally:
            stop(client)
            stop(server)
            if reader is not None:
                reader.join(timeout=5)
    saved.write_text(json.dumps(result, indent=2) + "\n")
    print(
        json.dumps(
            {
                key: result[key]
                for key in (
                    "configuration",
                    "connections",
                    "repeat",
                    "qps",
                    "p99_us",
                    "cpu_us_per_request",
                    "server_cpu_pct",
                    "client_cpu_pct",
                    "allocated_kib_per_request_with_warmup",
                )
            }
        ),
        flush=True,
    )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scoop-server", "go-server", "client", "work"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--server-cpus", type=integers, default=[0, 2, 4, 6])
    parser.add_argument("--client-cpus", type=integers, default=[8, 10, 12, 13, 14, 15, 16, 17])
    parser.add_argument("--workers", type=integers, default=[1, 4])
    parser.add_argument("--connections", type=integers, default=[1, 64])
    parser.add_argument("--payload", type=int, default=1024)
    parser.add_argument("--buffer", type=int, default=65536)
    parser.add_argument("--port", type=int, default=18082)
    parser.add_argument("--seconds", type=float, default=6)
    parser.add_argument("--warmup", type=float, default=1)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    assert not set(args.server_cpus) & set(args.client_cpus), "CPU sets overlap"
    assert args.workers and min(args.workers) > 0 and max(args.workers) <= len(args.server_cpus)
    assert min(args.connections) > 0 and args.payload >= 8 and args.buffer > 0
    assert args.seconds > 0 and args.warmup >= 0 and args.repeats > 0
    args.work.mkdir(parents=True, exist_ok=True)
    configurations = [("scoop-owner", 0, 1, args.scoop_server)]
    for workers in args.workers:
        configurations.extend(
            [
                (f"scoop-{workers}", workers, workers, args.scoop_server),
                (f"go-{workers}", workers, workers, args.go_server),
            ]
        )
    matrix = [
        (configuration, connections, repeat)
        for repeat in range(args.repeats)
        for connections in args.connections
        for configuration in configurations
    ]
    random.Random(20261112).shuffle(matrix)
    rows = [run_sample(args, *item) for item in matrix]
    (args.work / "samples.json").write_text(json.dumps(rows, indent=2) + "\n")


if __name__ == "__main__":
    main()
