#!/usr/bin/env python3
"""bench.py — measure brailer's verify/render cost on the design-loop corpus.

This is the honest counterweight to the numbers in README.md: it re-runs the
same measurements on *your* hardware so the claimed timings are reproducible,
not one-off quotes from a developer laptop.

    python3 scripts/bench.py            # median of 3 runs per spec
    python3 scripts/bench.py --runs 9

The `render` step writes to a temp dir so the corpus stays untouched.
"""

import argparse
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

BRAILER = Path("target/release/brailer")
SPECS = sorted(Path("tests/corpus").glob("*.json"))


def time_cmd(cmd: list[str], runs: int) -> float:
    """Median wall time (seconds) of running `cmd` `runs` times."""
    times = []
    for _ in range(runs):
        t0 = time.perf_counter()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        times.append(time.perf_counter() - t0)
    return statistics.median(times)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--render-spec", default=None,
                    help="spec to render (default: the smallest example); "
                         "'all' renders every example, 'none' skips render")
    args = ap.parse_args()
    if not BRAILER.exists():
        print("error: build first: cargo build --release", file=sys.stderr)
        return 2
    if not SPECS:
        print("error: no corpus specs found", file=sys.stderr)
        return 2

    smallest = min(SPECS, key=lambda p: p.stat().st_size)
    if args.render_spec == "all":
        render_specs = SPECS
    elif args.render_spec == "none":
        render_specs = []
    elif args.render_spec:
        render_specs = [Path(args.render_spec)]
    else:
        render_specs = [smallest]

    print(f"{'spec':<28} {'verify':>10} {'render@1x':>12} {'render@2x':>12}")
    print("-" * 66)
    for spec in SPECS:
        verify = time_cmd([str(BRAILER), "verify", str(spec)], args.runs)
        if spec in render_specs:
            with tempfile.TemporaryDirectory() as out:
                r1 = time_cmd([str(BRAILER), "render", str(spec), "-o", out],
                              args.runs)
                r2 = time_cmd([str(BRAILER), "render", str(spec), "-o", out,
                               "--retina"], args.runs)
            print(f"{spec.name:<28} {verify*1000:>8.0f} ms {r1:>10.2f} s "
                  f"{r2:>10.2f} s")
        else:
            print(f"{spec.name:<28} {verify*1000:>8.0f} ms {'—':>12} {'—':>12}")
    print("-" * 66)
    print("render = all frames of one spec; @2x = retina output. Medians on "
          "this machine. Pass --render-spec all to measure every render.")
    return 0


if __name__ == "__main__":
    sys.exit(main())