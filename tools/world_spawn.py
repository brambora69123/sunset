#!/usr/bin/env python3
"""Print a vanilla world's spawn point from its level.dat.

Usage: python3 tools/world_spawn.py /path/to/world
Prints "spawnX spawnY spawnZ". The spawn chunk is the one the server
populates first, which makes it the chunk to compare a generator against.
"""
import gzip
import struct
import sys


def read_nbt(data):
    pos = 0

    def u(fmt):
        nonlocal pos
        size = struct.calcsize(fmt)
        value = struct.unpack_from(fmt, data, pos)[0]
        pos += size
        return value

    def name():
        nonlocal pos
        length = struct.unpack_from(">H", data, pos)[0]
        pos += 2
        text = data[pos:pos + length].decode("utf-8", "replace")
        pos += length
        return text

    def payload(tag):
        nonlocal pos
        if tag == 1:
            return u(">b")
        if tag == 2:
            return u(">h")
        if tag == 3:
            return u(">i")
        if tag == 4:
            return u(">q")
        if tag == 5:
            return u(">f")
        if tag == 6:
            return u(">d")
        if tag == 7:
            n = u(">i")
            pos += n
            return None
        if tag == 8:
            n = u(">H")
            text = data[pos:pos + n].decode("utf-8", "replace")
            pos += n
            return text
        if tag == 9:
            item = data[pos]
            pos += 1
            n = u(">i")
            return [payload(item) for _ in range(n)]
        if tag == 10:
            out = {}
            while True:
                item = data[pos]
                pos += 1
                if item == 0:
                    return out
                key = name()
                out[key] = payload(item)
        if tag == 11:
            n = u(">i")
            pos += 4 * n
            return None
        if tag == 12:
            n = u(">i")
            pos += 8 * n
            return None
        raise ValueError(f"tag {tag}")

    tag = u(">b")
    name()
    return payload(tag)


def main():
    world = sys.argv[1]
    path = f"{world}/level.dat" if not world.endswith(".dat") else world
    with gzip.open(path, "rb") as f:
        root = read_nbt(f.read())
    data = root["Data"]
    print(data["SpawnX"], data["SpawnY"], data["SpawnZ"])


if __name__ == "__main__":
    main()
