#!/bin/sh
# Every gate a claim must pass, in one place.
#
#   MAP=/tmp/revimap.jsonl EXE=~/leonida/drop/exe/game_scarlett_master.exe tools/check-all.sh
#
# Order matters: the cheap structural checks first, the compile last.
set -u
MAP=${MAP:-/tmp/revimap.jsonl}
EXE=${EXE:-$HOME/leonida/drop/exe/game_scarlett_master.exe}
REVI=${REVI:-$HOME/revi-map-src/target/release/revi}
rc=0

echo "== claims, notes, src, tests"
"$REVI" check "$MAP" decomp || rc=1

echo "== traps"
python3 tools/oracle/traps.py --map "$MAP" decomp/traps/*.jsonl || rc=1

echo "== vectors"
if ls tools/oracle/examples/*.jsonl >/dev/null 2>&1; then
    python3 tools/oracle/validate.py --map "$MAP" tools/oracle/examples/*.jsonl || rc=1
    if [ -x "$REVI" ] && [ -f "$EXE" ]; then
        python3 tools/oracle/replay.py tools/oracle/examples/smoke.jsonl \
            --exe "$EXE" --revi "$REVI" || rc=1
    fi
fi

echo "== sabotage"
if [ -x "$REVI" ] && [ -f "$EXE" ]; then
    sh tools/oracle/sabotage.sh "$EXE" "$REVI" || rc=1
fi

echo "== compile"
REVI_CC=${REVI_CC:-$HOME/srv/revi/bin/cc} tools/build-src.sh --strict || rc=1

echo "== swap"
if [ -x "$REVI" ] && [ -f "$EXE" ] && [ -f target/src-obj/0x001117e0.obj ]; then
    sh tools/oracle/swap.sh "$EXE" "$REVI" target/src-obj $((0x1117e0)) 22 16 2>/dev/null || rc=1
fi

echo
[ "$rc" = 0 ] && echo "all gates green" || echo "gates failed"
exit "$rc"
