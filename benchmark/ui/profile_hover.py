#!/usr/bin/env python3
"""Build and run the macOS shadcn hover workload against a prepared sysroot."""

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
from pathlib import Path


def observation(executable, environment, log):
    result = subprocess.run(
        [str(executable)], capture_output=True, text=True, env=environment, check=True
    )
    log.write_text(result.stdout + result.stderr)
    motion = re.search(r"motion_frames=(\d+)", result.stdout)
    idle = re.search(r"idle_frames=(\d+)", result.stdout)
    counters = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    if not motion or not idle or len(counters) != 2:
        raise RuntimeError(f"incomplete hover observation; inspect {log}")
    frames = int(motion.group(1))
    if frames == 0 or counters[0]["completed"] != counters[0]["buffers"]:
        raise RuntimeError(f"motion work has not completed; inspect {log}")
    return {
        "frames": frames,
        "passes_per_frame": counters[0]["passes"] / frames,
        "target_pixels_per_frame": counters[0]["target_pixels"] / frames,
        "gpu_ms_per_frame": counters[0]["gpu_ms"] / frames,
        "idle_frames": int(idle.group(1)),
        "idle_buffers": counters[1]["buffers"],
    }


def summarize(samples):
    return {
        "samples": samples,
        "median": {
            key: statistics.median(sample[key] for sample in samples)
            for key in ("passes_per_frame", "target_pixels_per_frame", "gpu_ms_per_frame")
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument("--scoop", type=Path)
    parser.add_argument("--sysroot", type=Path)
    parser.add_argument("--cache-dir", type=Path)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--trace-passes", action="store_true")
    parser.add_argument(
        "--baseline", type=Path, help="alternate runs with a previously built probe"
    )
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this observation uses macOS Metal command buffers")
    if args.runs < 1:
        parser.error("--runs must be positive")
    if args.baseline and not args.baseline.is_file():
        parser.error("--baseline must name a previously built probe")
    root = Path(__file__).resolve().parents[2]
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    cone = work / "hover"
    cone.mkdir(exist_ok=True)
    source = cone / "src"
    if not source.exists():
        source.symlink_to(root / "sysroot/src/scoop.ui.component/examples/shadcn/src")
    assets = Path(__file__).parent / "hover"
    for name in ("probe.scoop", "profile.m"):
        content = (assets / name).read_bytes()
        destination = cone / name
        if not destination.exists() or destination.read_bytes() != content:
            destination.write_bytes(content)
    manifest = (
        (assets / "Cone.template.toml")
        .read_text()
        .replace("@SURFACE_TEST_NATIVE@", str(root / "sysroot/src/scoop.surface/test-native"))
    )
    destination = cone / "Cone.toml"
    if not destination.exists() or destination.read_text() != manifest:
        destination.write_text(manifest)
    executable = work / "hover-profile"
    command = [
        str((args.scoop or root / "target/release/scoop").resolve()),
        "build",
        str(cone),
        "--release",
        "--sysroot",
        str((args.sysroot or root / "sysroot").resolve()),
        "--cache-dir",
        str((args.cache_dir or root / "tmp/sysroot/aarch64-apple-darwin/cache").resolve()),
        "--target-dir",
        str(work / "target"),
        "-o",
        str(executable),
    ]
    with (work / "build.log").open("w") as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    environment = os.environ.copy()
    if args.trace_passes:
        environment["SCOOP_UI_GPU_TRACE"] = "1"
    samples = []
    baseline = []
    for run in range(args.runs):
        programs = [("after", executable, samples)]
        if args.baseline:
            programs.insert(0, ("before", args.baseline.resolve(), baseline))
            if run % 2:
                programs.reverse()
        for name, program, destination in programs:
            sample = observation(program, environment, work / f"{name}-{run + 1}.log")
            destination.append(sample)
            print(json.dumps({"stage": name, **sample}), flush=True)
    report = summarize(samples)
    if baseline:
        report["baseline"] = summarize(baseline)
        report["ratio_after_before"] = {
            key: value / report["baseline"]["median"][key]
            for key, value in report["median"].items()
            if report["baseline"]["median"][key] > 0
        }
    (work / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"median": report["median"]}), flush=True)


if __name__ == "__main__":
    main()
