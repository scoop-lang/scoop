import contextlib
import socket
import subprocess
import sys
import threading

import http1_peer


def consume(binary, mode, *urls):
    result = subprocess.run(
        [binary, mode, *urls],
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert result.returncode == 0, (mode, urls, result.returncode, result.stdout, result.stderr)
    assert result.stdout == f"client Basic {mode}\n" and not result.stderr, result


def exercise(binary, mode, serve):
    failures = []
    with contextlib.ExitStack() as stack:
        listeners = [stack.enter_context(socket.socket()) for _ in range(4)]
        for listener in listeners:
            listener.bind(("127.0.0.1", 0))
            listener.listen(4)
            listener.settimeout(45)

        def worker(first, second):
            try:
                serve(first, second)
            except BaseException as error:
                failures.append(error)

        workers = [
            threading.Thread(target=worker, args=pair, daemon=True)
            for pair in (listeners[:2], listeners[2:])
        ]
        for thread in workers:
            thread.start()
        consume(
            binary,
            mode,
            *[f"http://127.0.0.1:{s.getsockname()[1]}/private/start" for s in listeners],
        )
        for thread in workers:
            thread.join(45)
            assert not thread.is_alive(), (mode, "authentication peer did not finish")
        assert not failures, (mode, failures)
        for listener in listeners:
            listener.settimeout(0.05)
            try:
                extra, _ = listener.accept()
            except TimeoutError:
                continue
            extra.close()
            raise AssertionError((mode, "unexpected additional connection"))


def main():
    exercise(sys.argv[1], "challenge", http1_peer.challenge)
    exercise(sys.argv[1], "redirect", http1_peer.redirect)
    print("independent Basic HTTP1 challenge and redirect protocol checks passed")


if __name__ == "__main__":
    main()
