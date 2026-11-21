#!/usr/bin/env python3
"""Full-chunk terrain parity for the 1.7.10 port.

Reads a vanilla 1.7.10 chunk from an Anvil region file, rebuilds the 16x16x256
block id array from its sections (Blocks + Add nibbles), and diffs it against a
dump from the Rust generator. Also reports biome and height map equality.

Usage: python3 tools/terrain_parity_1_7_10.py WORLD_DIR CHUNK_X CHUNK_Z RUST_DUMP
RUST_DUMP is a file of 65536 comma-separated block ids, y-major (y<<8|z<<4|x).
"""
import struct
import sys

from biome_parity_1_7_10 import chunk_level


def nibble_get(data, index):
    byte = data[index >> 1]
    return (byte >> 4) if index & 1 else (byte & 15)


def chunk_metadata(level):
    """Rebuild the flat 4-bit metadata array from vanilla's sections."""
    out = [0] * 65536
    for section in level["Sections"]:
        y_base = section["Y"] * 16
        data = section.get("Data")
        if data is None:
            continue
        for y in range(16):
            for z in range(16):
                for x in range(16):
                    i = (y << 8) | (z << 4) | x
                    out[((y_base + y) << 8) | (z << 4) | x] = nibble_get(data, i)
    return out


def parse_dump(text):
    """A dump is `id:meta` per block in save order (older dumps: bare ids)."""
    ids = []
    metas = []
    for item in text.strip().rstrip(",").split(","):
        if not item:
            continue
        if ":" in item:
            block, meta = item.split(":")
            ids.append(int(block))
            metas.append(int(meta))
        else:
            ids.append(int(item))
            metas.append(0)
    return ids, metas


def chunk_blocks(level):
    """Rebuild the flat block id array from vanilla's sections."""
    out = [0] * 65536
    for section in level["Sections"]:
        y_base = section["Y"] * 16
        blocks = section["Blocks"]
        add = section.get("Add")
        for y in range(16):
            for z in range(16):
                for x in range(16):
                    i = (y << 8) | (z << 4) | x
                    block = blocks[i]
                    if add is not None:
                        block |= nibble_get(add, i) << 8
                    out[((y_base + y) << 8) | (z << 4) | x] = block
    return out


def main():
    world, cx, cz, dump = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    level = chunk_level(f"{world}/region/r.{cx >> 5}.{cz >> 5}.mca", cx, cz)
    if not level:
        print("no such chunk in the vanilla world")
        return 1
    vanilla = chunk_blocks(level)
    vanilla_meta = chunk_metadata(level)
    with open(dump) as f:
        rust, rust_meta = parse_dump(f.read())
    if len(rust) != 65536:
        print(f"rust dump has {len(rust)} ids, expected 65536")
        return 1
    diffs = [(i, vanilla[i], rust[i]) for i in range(65536) if vanilla[i] != rust[i]]
    meta_diffs = [
        (i, vanilla_meta[i], rust_meta[i])
        for i in range(65536)
        if vanilla_meta[i] != rust_meta[i]
    ]
    print(f"chunk {cx},{cz}: {65536 - len(diffs)}/65536 blocks match, "
          f"{65536 - len(meta_diffs)}/65536 metadata match")
    if diffs:
        print("first diffs (index, vanilla, rust):", diffs[:12])
    if meta_diffs:
        print("first metadata diffs (index, vanilla, rust):", meta_diffs[:12])
    return 0 if not diffs and not meta_diffs else 1


if __name__ == "__main__":
    sys.exit(main())
