"""Exercise real downloads, concurrent reuse and interrupted publication."""

import hashlib
import http.server
import io
import os
import sys
import tarfile
import tempfile
import threading
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import download_support
import package_llvm


class DownloadTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="downloads with spaces-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.requests = []
        self.body = b"fixed upstream input"
        self.interrupted = False
        owner = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                owner.requests.append(self.path)
                self.send_response(200)
                self.send_header(
                    "Content-Length", str(len(owner.body) + (10 if owner.interrupted else 0))
                )
                self.end_headers()
                self.wfile.write(owner.body)

            def log_message(self, *args):
                pass

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.url = f"http://127.0.0.1:{server.server_port}/input"

    def obtain(self, version="1", **kwargs):
        return download_support.obtain(
            self.root / "downloads", "sample", version, "input", self.url, **kwargs
        )

    def test_concurrent_download_and_later_hit_do_not_request_or_hash_again(self):
        with ThreadPoolExecutor(max_workers=4) as pool:
            paths = list(
                pool.map(
                    lambda _: self.obtain(sha256=hashlib.sha256(self.body).hexdigest()), range(4)
                )
            )
        self.assertEqual(len(set(paths)), 1)
        self.assertEqual(paths[0].read_bytes(), self.body)
        with patch.object(
            download_support.hashlib, "sha256", side_effect=AssertionError("cache was hashed")
        ):
            self.assertEqual(self.obtain(sha256="unused on hit"), paths[0])
        self.assertEqual(len(self.requests), 1)
        self.obtain("2")
        self.assertEqual(len(self.requests), 2)
        self.assertTrue(paths[0].is_file())

    def test_interrupted_and_wrong_digest_downloads_do_not_publish(self):
        self.interrupted = True
        with self.assertRaises((ValueError, http.client.IncompleteRead)):
            self.obtain()
        final = self.root / "downloads/sample/1/input"
        self.assertFalse(final.exists())
        self.assertFalse(final.with_name("input.part").exists())
        self.interrupted = False
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            self.obtain(sha256="0" * 64)
        self.assertFalse(final.exists())
        self.assertEqual(self.obtain().read_bytes(), self.body)

    def test_directory_precedence_and_local_fallback(self):
        with patch.dict(os.environ, {"SCOOP_DOWNLOAD_DIR": str(self.root / "environment")}):
            self.assertEqual(download_support.download_directory(), self.root / "environment")
            self.assertEqual(
                download_support.download_directory(self.root / "explicit"), self.root / "explicit"
            )
        local = self.root / "vendor-input"
        local.write_bytes(self.body)
        self.assertEqual(self.obtain(local=local), local)
        self.assertEqual(self.requests, [])
        cached = self.obtain()
        self.assertEqual(self.obtain(local=local), cached)
        self.assertTrue(local.is_file())

    def test_two_llvm_work_directories_share_the_original_archive(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w:xz") as archive:
            entry = tarfile.TarInfo(f"llvm-project-{package_llvm.VERSION}.src/llvm/source.cpp")
            entry.size = len(self.body)
            archive.addfile(entry, io.BytesIO(self.body))
        self.body = data.getvalue()

        def from_server(directory, dependency, version, filename, url, **kwargs):
            return download_support.obtain(
                directory, dependency, version, filename, self.url, **kwargs
            )

        with (
            patch.object(package_llvm, "ROOT", self.root),
            patch.object(package_llvm, "obtain", side_effect=from_server),
        ):
            first, archive = package_llvm.prepare(
                self.root / "work one", None, None, self.root / "downloads"
            )
            second, other_archive = package_llvm.prepare(
                self.root / "work two", None, None, self.root / "downloads"
            )
            again, _ = package_llvm.prepare(
                self.root / "work one", None, None, self.root / "downloads"
            )
        self.assertEqual(len(self.requests), 1)
        self.assertEqual(archive, other_archive)
        self.assertEqual(first, again)
        self.assertNotEqual(first, second)
        self.assertEqual(
            (first / "llvm/source.cpp").read_bytes(), (second / "llvm/source.cpp").read_bytes()
        )
        self.assertTrue(archive.is_file())


if __name__ == "__main__":
    unittest.main()
