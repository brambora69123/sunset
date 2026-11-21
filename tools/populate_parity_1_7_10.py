#!/usr/bin/env python3
"""Population parity for the 1.7.10 port, end to end.

Runs the Java oracle (`StageDump`, the real `biome.decorate` including every
per-biome override) and the Rust probe (`stage_dump`) for one chunk, compares
the stage hashes and the draw stream after decoration, then the blocks.

Usage:
  python3 tools/populate_parity_1_7_10.py SEED CHUNK_X CHUNK_Z [--blocks]

Requires:
  sh tools/javaoracle_1_7_10/build.sh      (once)
  cargo +1.99.1 build --release -p sunset-1-7-chunkgen --bin stage_dump
"""
import os
import subprocess
import sys

LINES = ("start", "water", "lava", "dungeon", "decorate", "rngafter", "snow")


def java(seed, cx, cz, mode=None):
    args = ["sh", "tools/javaoracle_1_7_10/run.sh", "StageDump", str(seed), str(cx), str(cz)]
    if mode:
        args.append(mode)
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout.splitlines()


BIN = os.path.join("target", "release", "stage_dump")


def rust(seed, cx, cz, mode=None):
    """Runs the already-built probe; building it once is far faster than a
    `cargo run` per call, which recompiles after every edit to the crate."""
    if not os.path.exists(BIN):
        subprocess.run(["cargo", "+1.99.1", "build", "--release",
                        "-p", "sunset-1-7-chunkgen", "--bin", "stage_dump"],
                       check=True)
    args = [BIN, str(seed), str(cx), str(cz)]
    if mode:
        args.append(mode)
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout.splitlines()


def indexed(lines, prefix):
    for line in lines:
        if line.startswith(prefix + " "):
            return line[len(prefix) + 1:]
    return None


def main():
    seed, cx, cz = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    check_blocks = "--blocks" in sys.argv

    want = java(seed, cx, cz)
    got = rust(seed, cx, cz)
    bad = 0
    for name in LINES:
        a, b = indexed(want, name), indexed(got, name)
        if a is None or b is None:
            print(f"{name}: missing line (java={a!r} rust={b!r})")
            bad += 1
        elif a != b:
            print(f"{name}: differs")
            print(f"  java: {a}")
            print(f"  rust: {b}")
            bad += 1
        else:
            print(f"{name}: same")

    if check_blocks:
        jb = indexed(java(seed, cx, cz, "blocks"), "BLOCKS")
        rb = indexed(rust(seed, cx, cz, "blocks"), "BLOCKS")
        if jb == rb:
            print("blocks after decoration: identical")
        else:
            w = [int(v.split(":")[0]) for v in jb.split(",")]
            g = [int(v.split(":")[0]) for v in rb.split(",")]
            diffs = [(i, a, b) for i, (a, b) in enumerate(zip(w, g)) if a != b]
            print(f"blocks after decoration: {len(diffs)} differ, first:", diffs[:8])
            bad += 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
