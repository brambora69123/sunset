#!/usr/bin/env python3
"""Biome parity for the 1.7.10 port: vanilla server's saved chunk biomes vs sunset's probe.

Needs a vanilla 1.7.10 world generated with a known seed (see tools/vanilla_1_7_10_oracle.py).
Usage: python3 tools/biome_parity_1_7_10.py WORLD_DIR SEED [RANGE] [PROBE]
WORLD_DIR holds world/region/*.mca. RANGE is a chunk coordinate radius (default 8).
"""
import collections
import gzip
import struct
import subprocess
import sys


class Reader:
    def __init__(self, data):
        self.d, self.i = data, 0

    def u1(self):
        v = self.d[self.i]
        self.i += 1
        return v

    def u2(self):
        v = struct.unpack_from(">H", self.d, self.i)[0]
        self.i += 2
        return v

    def i4(self):
        v = struct.unpack_from(">i", self.d, self.i)[0]
        self.i += 4
        return v

    def i8(self):
        v = struct.unpack_from(">q", self.d, self.i)[0]
        self.i += 8
        return v

    def f4(self):
        v = struct.unpack_from(">f", self.d, self.i)[0]
        self.i += 4
        return v

    def f8(self):
        v = struct.unpack_from(">d", self.d, self.i)[0]
        self.i += 8
        return v

    def short(self):
        v = struct.unpack_from(">h", self.d, self.i)[0]
        self.i += 2
        return v

    def text(self):
        n = self.u2()
        v = self.d[self.i:self.i + n].decode("utf-8", "replace")
        self.i += n
        return v

    def payload(self, t):
        if t == 1:
            return self.u1()
        if t == 2:
            return self.short()
        if t == 3:
            return self.i4()
        if t == 4:
            return self.i8()
        if t == 5:
            return self.f4()
        if t == 6:
            return self.f8()
        if t == 7:
            n = self.i4()
            v = list(self.d[self.i:self.i + n])
            self.i += n
            return v
        if t == 8:
            return self.text()
        if t == 9:
            et, n = self.u1(), self.i4()
            return [self.payload(et) for _ in range(n)]
        if t == 10:
            out = {}
            while True:
                ct = self.u1()
                if ct == 0:
                    return out
                name = self.text()
                out[name] = self.payload(ct)
        if t == 11:
            n = self.i4()
            v = list(struct.unpack_from(">%di" % n, self.d, self.i))
            self.i += 4 * n
            return v
        if t == 12:
            n = self.i4()
            v = list(struct.unpack_from(">%dq" % n, self.d, self.i))
            self.i += 8 * n
            return v
        raise ValueError("tag %d at %d" % (t, self.i))

    def tag(self):
        t = self.u1()
        if t == 0:
            return None, None
        name = self.text()
        return name, self.payload(t)


def chunk_level(path, cx, cz):
    with open(path, "rb") as f:
        raw = f.read()
    idx = (cx & 31) + (cz & 31) * 32
    off = struct.unpack_from(">I", raw, idx * 4)[0] >> 8
    if not off:
        return None
    off *= 4096
    length = struct.unpack_from(">I", raw, off)[0]
    comp = raw[off + 4]
    payload = raw[off + 5:off + 4 + length]
    if comp == 2:
        import zlib
        payload = zlib.decompress(payload)
    elif comp == 1:
        payload = gzip.decompress(payload)
    _, root = Reader(payload).tag()
    return root.get("Level")


def main():
    world, seed = sys.argv[1], sys.argv[2]
    radius = int(sys.argv[3]) if len(sys.argv) > 3 else 8
    probe = sys.argv[4] if len(sys.argv) > 4 else "./target/debug/biome_probe"
    ok, missing, pairs, worst = 0, 0, collections.Counter(), []
    for cx in range(-radius, radius + 1):
        for cz in range(-radius, radius + 1):
            level = chunk_level(f"{world}/region/r.{cx >> 5}.{cz >> 5}.mca", cx, cz)
            if not level or "Biomes" not in level:
                missing += 1
                continue
            expect = level["Biomes"]
            got = [int(v) for v in subprocess.check_output(
                [probe, seed, str(cx), str(cz)], text=True).strip().split(",")]
            if expect == got:
                ok += 1
            else:
                diffs = sum(a != b for a, b in zip(expect, got))
                worst.append((diffs, cx, cz))
                for a, b in zip(expect, got):
                    if a != b:
                        pairs[(a, b)] += 1
    print(f"full-match chunks: {ok}, mismatched: {len(worst)}, missing: {missing}")
    if worst:
        print("worst chunks:", sorted(worst, reverse=True)[:8])
        print("mismatch pairs (vanilla, sunset):", pairs.most_common(15))


if __name__ == "__main__":
    main()
