#!/bin/sh
# Runs a built 1.7.10 oracle probe: tools/javaoracle_1_7_10/run.sh PopulateStages 1234 0 0 7
set -eu
cd "$(dirname "$0")/../.."
OUT=.local/tools/1.7.10
# The render probe needs the client's Block/BlockRenderer classes first; the
# world and generator probes need the server's classes first.
case "${1:-}" in
    RenderOracle) CP="$OUT/oracle:$OUT/client-public.jar:$OUT/server-public.jar" ;;
    *) CP="$OUT/oracle:$OUT/server-public.jar:$OUT/client-public.jar" ;;
esac
exec java -cp "$CP" "$@"
