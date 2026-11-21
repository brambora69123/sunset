#!/usr/bin/env python3
"""Light parity for the 1.7.10 port: a vanilla server's saved skylight and
block light against the engine's light stage on the same generated chunk.

The probe (`sunset17_probe ... --light`) generates the chunk with
`--1-7-10-terrain`'s generator, finishes it as the engine's column and lights
it with the engine's `light_chunk` over its 3x3 neighbourhood, printing one
hex digit per block in 1.7.10's save order (`y << 8 | z << 4 | x`).

Needs a 1.7.10 world generated with a known seed (see
tools/vanilla_1_7_10_oracle.py). Build the probe with
`cargo build --no-default-features -p sunrise-client-lib --bin sunset17_probe`.
Usage:

  python3 tools/light_parity_1_7_10.py WORLD_DIR SEED [options]

WORLD_DIR holds region/*.mca. The default centre is the seed-4242 spawn chunk
(12, 16) and the default range 4, i.e. the 9x9 chunks around it. Options:
  --center CX CZ     chunk to compare around (default 12 16)
  --range N          chunk radius around it (default 4)
  --probe PATH       the probe binary (default target/debug/sunset17_probe)
  --skip CX CZ       exclude a chunk (repeatable); listed as excluded

A chunk the server did not finish lighting (`LightPopulated` 0) is compared
but reported apart: its saved light can be incomplete, so it is not counted
toward the match. Chunks whose blocks the generator places differently can
still differ in light; the mismatch lines show where.
"""
import argparse
import os
import subprocess
import sys

from biome_parity_1_7_10 import chunk_level

CELLS = 16 * 16 * 256


def vanilla_lights(level):
    """The chunk's 65536 sky and block nibbles, in save order."""
    sky = [None] * CELLS
    block = [None] * CELLS
    for section in level["Sections"]:
        y_base = section["Y"] * 16
        for name, out in (("SkyLight", sky), ("BlockLight", block)):
            data = section.get(name)
            if data is None:
                continue
            for y in range(16):
                for z in range(16):
                    for x in range(16):
                        i = (y << 8) | (z << 4) | x
                        byte = data[i >> 1]
                        out[((y_base + y) << 8) | (z << 4) | x] = (
                            byte >> 4 if i & 1 else byte & 15
                        )
    # A section the chunk never made is not saved: the vanilla accessor answers
    # with the dimension default, which for skylight is 15 at and above the
    # column's height map and 0 below it, and 0 for block light.
    height_map = level.get("HeightMap") or []
    for y in range(256):
        for z in range(16):
            for x in range(16):
                i = (y << 8) | (z << 4) | x
                if sky[i] is None:
                    height = height_map[z * 16 + x] & 0xFF if height_map else 0
                    sky[i] = 15 if y >= height else 0
                if block[i] is None:
                    block[i] = 0
    return sky, block


def probe_lights(probe, seed, cx, cz):
    out = subprocess.check_output(
        [probe, str(seed), str(cx), str(cz), "1", "--light"], text=True
    )
    lines = out.split()
    values = {}
    for name, hexes in zip(lines[0::2], lines[1::2]):
        values[name] = [int(c, 16) for c in hexes]
    for name in ("sky", "block"):
        if len(values.get(name, ())) != CELLS:
            raise SystemExit(
                f"probe output for chunk {cx},{cz} has {len(values.get(name, ()))} "
                f"{name} nibbles, expected {CELLS}"
            )
    return values["sky"], values["block"]


def mismatches(expected, got):
    return [(i, expected[i], got[i]) for i in range(CELLS) if expected[i] != got[i]]


def position(i):
    return (i >> 8, (i >> 4) & 15, i & 15)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("world")
    parser.add_argument("seed", type=int)
    parser.add_argument("--center", nargs=2, type=int, default=(12, 16))
    parser.add_argument("--range", type=int, default=4)
    parser.add_argument("--probe", default="target/debug/sunset17_probe")
    parser.add_argument("--skip", nargs=2, type=int, action="append", default=[])
    args = parser.parse_args()

    skipped = {tuple(pair) for pair in args.skip}
    (ox, oz), radius = tuple(args.center), args.range
    rows = []
    missing = []
    for cx in range(ox - radius, ox + radius + 1):
        for cz in range(oz - radius, oz + radius + 1):
            path = f"{args.world}/region/r.{cx >> 5}.{cz >> 5}.mca"
            level = chunk_level(path, cx, cz) if os.path.isfile(path) else None
            if level is None or "Sections" not in level:
                missing.append((cx, cz))
                continue
            if (cx, cz) in skipped:
                print(f"chunk {cx},{cz}: skipped (--skip)")
                continue
            expected_sky, expected_block = vanilla_lights(level)
            got_sky, got_block = probe_lights(args.probe, args.seed, cx, cz)
            bad_sky = mismatches(expected_sky, got_sky)
            bad_block = mismatches(expected_block, got_block)
            rows.append(
                {
                    "chunk": (cx, cz),
                    "light_populated": bool(level.get("LightPopulated")),
                    "sky": bad_sky,
                    "block": bad_block,
                }
            )

    def report(rows, title, positions):
        print(title)
        sky_ok = sum(CELLS - len(r["sky"]) for r in rows)
        block_ok = sum(CELLS - len(r["block"]) for r in rows)
        cells = CELLS * len(rows)
        print(
            f"  {len(rows)} chunks, {cells} cells each light type: "
            f"sky {sky_ok}/{cells} ({100.0 * sky_ok / max(cells, 1):.4f}%), "
            f"block {block_ok}/{cells} ({100.0 * block_ok / max(cells, 1):.4f}%)"
        )
        exact = [r for r in rows if not r["sky"] and not r["block"]]
        print(f"  chunks exact for both: {len(exact)}/{len(rows)}")
        for r in rows:
            if not r["sky"] and not r["block"]:
                continue
            cx, cz = r["chunk"]
            print(
                f"  chunk {cx},{cz}: sky {CELLS - len(r['sky'])}/{CELLS}, "
                f"block {CELLS - len(r['block'])}/{CELLS}"
            )
            if not positions:
                continue
            for name in ("sky", "block"):
                for i, want, got in r[name][:5]:
                    y, z, x = position(i)
                    print(f"    {name} y={y} z={z} x={x}: vanilla {want}, engine {got}")
        return len(exact)

    lit = [r for r in rows if r["light_populated"]]
    unlit = [r for r in rows if not r["light_populated"]]
    exact = report(lit, "light-populated chunks", True)
    if unlit:
        report(unlit, "chunks the server left with LightPopulated 0 (not counted)", False)
    if missing:
        print(f"no saved chunk at: {', '.join(f'{x},{z}' for x, z in missing)}")
    if skipped:
        print(f"excluded by --skip: {', '.join(f'{x},{z}' for x, z in sorted(skipped))}")

    wanted = 20
    if exact < wanted:
        print(f"FAIL: {exact} exact chunks, at least {wanted} required")
        return 1
    print(f"PASS: {exact} exact chunks, at least {wanted} required")
    return 0


if __name__ == "__main__":
    sys.exit(main())
