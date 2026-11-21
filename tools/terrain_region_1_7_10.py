#!/usr/bin/env python3
"""Region-wide terrain parity for the 1.7.10 port.

Diffs every chunk of a dump directory (see
`region_dump` in crates/v17/chunkgen) against the vanilla world, and prints the
chunks grouped by exactness so a single bad biome stands out.

Usage:
  python3 tools/terrain_region_1_7_10.py WORLD_DIR DUMP_DIR [--worst N]

Chunks the vanilla world does not have are skipped.
"""
import os
import sys
from collections import Counter

from biome_parity_1_7_10 import chunk_level
from terrain_parity_1_7_10 import chunk_blocks, chunk_metadata, parse_dump


def main():
    world = sys.argv[1]
    dump_dir = sys.argv[2]
    worst = 12
    if "--worst" in sys.argv:
        worst = int(sys.argv[sys.argv.index("--worst") + 1])

    inner = None
    if "--inner" in sys.argv:
        i = sys.argv.index("--inner")
        inner = (int(sys.argv[i + 1]), int(sys.argv[i + 2]), int(sys.argv[i + 3]))

    rows = []
    for name in sorted(os.listdir(dump_dir)):
        if not name.endswith(".txt"):
            continue
        cx, cz = (int(v) for v in name[:-4].split("_"))
        if inner is not None:
            ox, oz, r = inner
            if max(abs(cx - ox), abs(cz - oz)) > r:
                continue
        level = chunk_level(f"{world}/region/r.{cx >> 5}.{cz >> 5}.mca", cx, cz)
        if level is None:
            continue
        vanilla = chunk_blocks(level)
        vanilla_meta = chunk_metadata(level)
        with open(os.path.join(dump_dir, name)) as f:
            rust, rust_meta = parse_dump(f.read())
        if len(rust) != 65536:
            print(f"{name}: dump has {len(rust)} ids, expected 65536")
            continue
        match = sum(1 for a, b in zip(vanilla, rust) if a == b)
        meta_match = sum(1 for a, b in zip(vanilla_meta, rust_meta) if a == b)
        rows.append((match / 65536.0, match, cx, cz, meta_match))

    if not rows:
        print("no comparable chunks")
        return 1

    total = sum(r[1] for r in rows)
    meta_total = sum(r[4] for r in rows)
    print(f"{len(rows)} chunks: {total} / {len(rows) * 65536} blocks match "
          f"({100.0 * total / (len(rows) * 65536):.4f}%), "
          f"{meta_total} / {len(rows) * 65536} metadata match "
          f"({100.0 * meta_total / (len(rows) * 65536):.4f}%)")
    buckets = Counter()
    for ratio, _, _, _, _ in rows:
        if ratio == 1.0:
            buckets["exact"] += 1
        elif ratio >= 0.9999:
            buckets["<0.01% off"] += 1
        elif ratio >= 0.999:
            buckets["<0.1% off"] += 1
        elif ratio >= 0.99:
            buckets["<1% off"] += 1
        elif ratio >= 0.95:
            buckets["<5% off"] += 1
        else:
            buckets["5%+ off"] += 1
    for label in ("exact", "<0.01% off", "<0.1% off", "<1% off", "<5% off", "5%+ off"):
        if buckets[label]:
            print(f"  {label}: {buckets[label]}")
    rows.sort()
    print("worst chunks:")
    for ratio, match, cx, cz, meta_match in rows[:worst]:
        print(f"  chunk {cx},{cz}: {match}/65536 blocks ({100.0 * ratio:.3f}%), "
              f"{meta_match}/65536 metadata")
    return 0


if __name__ == "__main__":
    sys.exit(main())
