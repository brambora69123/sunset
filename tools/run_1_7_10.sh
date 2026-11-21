#!/bin/sh
# Play the 1.7.10 port with audio on, in a reusable scratch folder.
#
#   tools/run_1_7_10.sh --world Play --creative
#   tools/run_1_7_10.sh --seed 42 --render-distance 8
#
# The scratch folder (default ~/.s17-test) holds the game folder, its saves,
# options.txt, logs and the control socket; nothing touches your real
# ~/.local/share/sunrise, the launcher's data or the repository's saves.
# Point S17_SCRATCH somewhere else or delete it to start over:
#
#   S17_SCRATCH=/tmp/s17-fresh tools/run_1_7_10.sh
#
# While it runs, another terminal can drive it:
#
#   target/fast/sunrise-ctl --socket ~/.s17-test/ctl.sock state
#   target/fast/sunrise-ctl --socket ~/.s17-test/ctl.sock break X Y Z
#   target/fast/sunrise-ctl --socket ~/.s17-test/ctl.sock command "time set night"
#
# Audio is ON here (the agent runs of AGENTS.md set SUNRISE_NO_AUDIO=1; this
# script removes it), so breaking and walking should be audible.
set -eu
cd "$(dirname "$0")/.."
ROOT=$PWD
SCRATCH=${S17_SCRATCH:-$HOME/.s17-test}
GAME=$SCRATCH/game
RES=$ROOT/.local/sunrise-client/1.7.10
BIN=$ROOT/target/fast/sunrise-client

if [ ! -x "$BIN" ]; then
    echo "building the game (target/fast)..." >&2
    cargo build --profile fast -p sunrise-game
fi

# The era's content tables and assets: the extraction is idempotent, so this
# only does work after a jar or the Prism store changed.
if [ ! -f "$RES/facts.json" ] || [ ! -d "$RES/assets/minecraft/sounds" ]; then
    echo "preparing the 1.7.10 resources in $RES..." >&2
    cargo run -q -p sunrise-prepare -- assets-1.7.10 --root "$ROOT"
fi

mkdir -p "$SCRATCH/home" "$SCRATCH/launcher" "$SCRATCH/data" "$SCRATCH/config" "$GAME"

exec env -u SUNRISE_NO_AUDIO \
    SUNRISE_HOME="$SCRATCH/home" \
    SUNRISE_LAUNCHER_HOME="$SCRATCH/launcher" \
    XDG_DATA_HOME="$SCRATCH/data" \
    XDG_CONFIG_HOME="$SCRATCH/config" \
    SUNRISE_LAUNCHER_NO_SELF_UPDATE=1 \
    "$BIN" \
    --1-7-10-terrain \
    --resources "$RES" \
    --game-dir "$GAME" \
    --control-socket "$SCRATCH/ctl.sock" \
    "$@"
