#!/usr/bin/env python3
"""One command for a region-wide generation parity run.

Reads a vanilla world's spawn point, dumps the same area from the port, and
reports how many chunks match, with the worst offenders listed.

Usage:
  python3 tools/region_parity.py --world /tmp/vanilla1234/world --seed 1234 \
      [--radius 6] [--inner 4] [--dump /tmp/opencode/reg1234]

--radius is the populated radius around the spawn chunk, --inner how many
chunks around the centre are compared (the edge chunks have neighbours the
dump never populated, so comparing them understates parity).
"""
import argparse
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--world", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--radius", type=int, default=6)
    ap.add_argument("--inner", type=int)
    ap.add_argument("--dump")
    ap.add_argument("--worst", type=int, default=12)
    args = ap.parse_args()

    spawn = subprocess.run(
        [sys.executable, os.path.join(ROOT, "tools/world_spawn.py"), args.world],
        capture_output=True, text=True, check=True).stdout.split()
    cx, cz = int(spawn[0]) >> 4, int(spawn[2]) >> 4
    inner = args.radius - 2 if args.inner is None else args.inner
    dump = args.dump or f"/tmp/opencode/reg_{args.seed}_{cx}_{cz}"

    print(f"spawn chunk {cx},{cz}; dumping radius {args.radius} to {dump}")
    bin_path = os.path.join(ROOT, "target", "release", "region_dump")
    if not os.path.exists(bin_path):
        subprocess.run(
            ["cargo", "+1.99.1", "build", "--release",
             "-p", "sunset-1-7-chunkgen", "--bin", "region_dump"],
            cwd=ROOT, check=True)
    subprocess.run(
        [bin_path, str(args.seed), str(cx), str(cz), str(args.radius), dump],
        cwd=ROOT, check=True)

    result = subprocess.run(
        [sys.executable, os.path.join(ROOT, "tools/terrain_region_1_7_10.py"),
         args.world, dump, "--inner", str(cx), str(cz), str(inner),
         "--worst", str(args.worst)],
        cwd=ROOT, text=True)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
