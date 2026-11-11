"""Validate fixed-count TCP echoes and capture allocation probes, without timing claims."""

import argparse
import json
import socket
import subprocess
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--requests", type=int, default=16000)
    parser.add_argument("--port", type=int, default=18083)
    args = parser.parse_args()
    if args.requests <= 1000:
        parser.error("requests must exceed the probe's 1000-iteration warmup")
    args.work.mkdir(parents=True, exist_ok=False)
    stdout = args.work / "server.stdout"
    stderr = args.work / "server.stderr"
    command = ["taskset", "-c", "0", str(args.binary.resolve()), "1", str(args.port), "1", "65536"]
    with stdout.open("w") as output, stderr.open("w") as errors:
        process = subprocess.Popen(command, stdout=output, stderr=errors)
        try:
            deadline = time.monotonic() + 20
            while "READY\n" not in stdout.read_text():
                if process.poll() is not None:
                    raise RuntimeError(f"server exited during startup: {stderr.read_text()}")
                if time.monotonic() > deadline:
                    raise TimeoutError("server did not become ready")
                time.sleep(0.01)
            with socket.create_connection(("127.0.0.1", args.port), timeout=20) as stream:
                stream.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                for sequence in range(args.requests):
                    payload = sequence.to_bytes(8, "little") + bytes([sequence % 251]) * 1016
                    stream.sendall(payload)
                    response = bytearray()
                    while len(response) < len(payload):
                        part = stream.recv(len(payload) - len(response))
                        if not part:
                            raise EOFError("server closed before completing an echo")
                        response.extend(part)
                    if response != payload:
                        raise ValueError(f"echo body mismatch at request {sequence}")
                stream.shutdown(socket.SHUT_WR)
            status = process.wait(timeout=30)
            if status != 0:
                raise RuntimeError(f"server exited {status}: {stderr.read_text()}")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    lines = stdout.read_text().splitlines()
    metrics = [
        json.loads(line.removeprefix("METRICS ")) for line in lines if line.startswith("METRICS ")
    ]
    if len(metrics) != 2:
        raise ValueError("expected startup and shutdown metrics")
    probes = [
        json.loads(line.removeprefix("ALLOC_PHASES "))
        for line in lines
        if line.startswith("ALLOC_PHASES ")
    ]
    probe = probes[0] if len(probes) == 1 else None
    if probes and len(probes) != 1:
        raise ValueError("expected one completed allocation report")
    if probe is not None:
        if (
            probe["completed"] != args.requests
            or probe["measured"] != args.requests - probe["warmup"]
        ):
            raise ValueError("probe iterations differ from validated exchanges")
        if sum(phase["bytes"] for phase in probe["phases"]) != probe["allocated_bytes"]:
            raise ValueError("exclusive phase bytes do not sum to total allocation")
    result = {
        "command": command,
        "validated_exchanges": args.requests,
        "payload_bytes": 1024,
        "status": "ok",
        "lifetime_bytes_per_exchange": (
            metrics[1]["allocated_bytes"] - metrics[0]["allocated_bytes"]
        )
        / args.requests,
        "probe": probe,
    }
    (args.work / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key != "probe"}))


if __name__ == "__main__":
    main()
