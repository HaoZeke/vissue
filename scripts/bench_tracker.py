#!/usr/bin/env python3
"""What each verb costs on a tracker of a given size, and where the cost sits.

Rendering rows and computing the answer are different questions: `count` does
the parse and prints one number, `list` prints every row. Timing both separates
the two.

    python3 scripts/bench_tracker.py target/release/vissue [issues]
"""
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

WORDS = ("parser", "header", "overlay", "ripgrep", "review", "manifest", "record", "token")


def main(argv: list[str]) -> int:
    binary = Path(argv[0]) if argv else Path("target/release/vissue")
    total = int(argv[1]) if len(argv) > 1 else 5000
    root = Path(tempfile.mkdtemp(prefix="vissue-bench-"))
    (root / "Software").mkdir(parents=True)

    def run(*args: str) -> str:
        done = subprocess.run(
            [str(binary), "--root", str(root), *args], capture_output=True, text=True
        )
        return done.stdout

    def timed(call, reps: int = 7) -> float:
        times = []
        for _ in range(reps):
            start = time.perf_counter()
            call()
            times.append(time.perf_counter() - start)
        times.sort()
        return times[len(times) // 2] * 1000

    try:
        first = None
        for i in range(total):
            word = WORDS[i % len(WORDS)]
            out = run("create", "-p", f"proj{i % 4}", f"The {word} number {i} settled it", "-q")
            first = first or out.strip()
            # A third of the corpus waits on the first issue, so readiness has
            # something to decide rather than answering yes for everything.
            if i % 3 == 0 and first and out.strip() != first:
                run("update", out.strip(), "--block", first)

        rows = len(run("list").splitlines())
        ready_rows = len(run("ready").splitlines())
        print(f"{total} issues, {rows} rows listed, {ready_rows} ready")
        startup = timed(lambda: subprocess.run([str(binary), "--version"], capture_output=True))
        print(f"  {'process startup':18} {startup:6.1f} ms")
        for label, args in (
            ("count", ("count",)),
            ("count --ready", ("count", "--ready")),
            ("ready", ("ready",)),
            ("list", ("list",)),
            ("check", ("check",)),
            ("search", ("search", "parser")),
            ("tree", ("tree", first)),
            ("related", ("related", first)),
        ):
            print(f"  {label:18} {timed(lambda a=args: run(*a)):6.1f} ms")
        return 0
    finally:
        shutil.rmtree(root, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
