"""Reuse fixed upstream files independently of a build's working directory."""

import hashlib
import os
import urllib.request
from pathlib import Path

from build_support import ROOT, build_lock


def download_directory(explicit=None):
    return Path(
        explicit or os.environ.get("SCOOP_DOWNLOAD_DIR") or ROOT / "tmp/downloads"
    ).resolve()


def obtain(directory, dependency, version, filename, url, *, local=None, sha256=None):
    """Return a complete input, downloading only when no local input exists."""
    destination = directory / dependency / version / filename
    if destination.is_file():
        return destination
    if local is not None and local.is_file():
        return local.resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    with build_lock(destination.with_name(filename + ".lock")):
        if destination.is_file():
            return destination
        staged = destination.with_name(filename + ".part")
        digest = hashlib.sha256() if sha256 is not None else None
        print(f"Downloading {url}", flush=True)
        try:
            with urllib.request.urlopen(url, timeout=60) as response, staged.open("wb") as output:
                size = 0
                while block := response.read(1024 * 1024):
                    output.write(block)
                    size += len(block)
                    if digest is not None:
                        digest.update(block)
                expected = response.headers.get("Content-Length")
                if expected is not None and size != int(expected):
                    raise ValueError(f"Incomplete download: {url} ({size} of {expected} bytes)")
            if digest is not None and digest.hexdigest() != sha256:
                raise ValueError(f"SHA-256 mismatch for {url}")
            staged.replace(destination)
        finally:
            staged.unlink(missing_ok=True)
    return destination
