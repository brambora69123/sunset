#!/usr/bin/env python3
"""Generate a vanilla Minecraft world for parity work.

Starts the real server jar for a version, waits for the spawn area to finish,
stops it, and prints the world directory. That is the reference every
generation probe in this repo is diffed against.

Usage:
  python3 tools/vanilla_world.py --seed 1234 --out /tmp/world1234
  python3 tools/vanilla_world.py --seed 999 --no-structures --out /tmp/world999

The server jar comes from --jar, else the Fabric Loom cache for --version
(default 1.7.10). Java 8 is used when found, because the 1.7.10 server needs it.
"""
import argparse
import glob
import os
import shutil
import subprocess
import sys
import time


def find_jar(version):
    pattern = os.path.expanduser(
        f"~/.gradle/caches/fabric-loom/{version}/minecraft-server.jar"
    )
    hits = glob.glob(pattern)
    return hits[0] if hits else None


def find_java(version):
    if version.startswith("1.7") or version.startswith("1.6") or version.startswith("1.5"):
        for candidate in ("/usr/lib/jvm/java-8-openjdk/jre/bin/java",
                          "/usr/lib/jvm/java-8-openjdk/bin/java"):
            if os.path.exists(candidate):
                return candidate
    return shutil.which("java")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--version", default="1.7.10")
    ap.add_argument("--jar")
    ap.add_argument("--no-structures", action="store_true")
    ap.add_argument("--view-distance", type=int, default=10)
    ap.add_argument("--max-players", type=int, default=1)
    ap.add_argument("--timeout", type=int, default=180, help="startup budget, seconds")
    args = ap.parse_args()

    jar = args.jar or find_jar(args.version)
    if not jar or not os.path.exists(jar):
        sys.exit(f"no server jar for {args.version}; pass --jar")
    java = find_java(args.version)
    if not java:
        sys.exit("no java found")

    out = os.path.abspath(args.out)
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, "eula.txt"), "w") as f:
        f.write("eula=true\n")
    with open(os.path.join(out, "server.properties"), "w") as f:
        f.write(
            f"level-seed={args.seed}\n"
            "level-type=DEFAULT\n"
            f"generate-structures={'false' if args.no_structures else 'true'}\n"
            "spawn-protection=0\n"
            f"max-players={args.max_players}\n"
            f"view-distance={args.view_distance}\n"
        )

    log_path = os.path.join(out, "server.log")
    with open(log_path, "w") as log:
        proc = subprocess.Popen(
            [java, "-Xmx1G", "-jar", jar, "nogui"],
            cwd=out, stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT,
            text=True,
        )
        started = time.time()
        while time.time() - started < args.timeout:
            if proc.poll() is not None:
                break
            with open(log_path) as f:
                if "[Server thread/INFO]: Done" in f.read():
                    break
            time.sleep(0.5)
        else:
            proc.kill()
            sys.exit(f"server did not finish the spawn area within {args.timeout}s; see {log_path}")

        try:
            proc.stdin.write("stop\n")
            proc.stdin.flush()
        except (BrokenPipeError, ValueError):
            pass
        proc.wait(timeout=120)

    regions = sorted(glob.glob(os.path.join(out, "world", "region", "*.mca")))
    if not regions:
        sys.exit(f"server wrote no region files; see {log_path}")
    print(os.path.join(out, "world"))


if __name__ == "__main__":
    main()
