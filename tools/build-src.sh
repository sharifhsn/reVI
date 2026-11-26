#!/bin/sh
# Compile every decompiled body, one translation unit per file.
#
#   REVI_CC=~/srv/revi/bin/cc tools/build-src.sh [--strict]
#
# The pinned compiler wrapper (msvc-wine) enforces the language decision:
# C++20, /GR-, /EHsc, /GS- by default, /O2. Each file compiles alone, into
# target/, so one broken body never hides the others. --strict makes any
# failure fatal; without it the failures are listed and the exit is 0.
set -u
CC=${REVI_CC:-$HOME/srv/revi/bin/cc}
SRC=${SRC:-decomp/src}
OUT=${OUT:-target/src-obj}
STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

mkdir -p "$OUT"
ok=0
fail=0
failed=""
for f in "$SRC"/*.cpp; do
    [ -e "$f" ] || { echo "no sources in $SRC"; exit 2; }
    base=$(basename "$f" .cpp)
    if "$CC" /nologo /c /std:c++20 /GR- /EHsc /GS- /O2 /W3 \
        /Fo:"$OUT/$base.obj" "$f" >"$OUT/$base.log" 2>&1; then
        ok=$((ok + 1))
    else
        fail=$((fail + 1))
        failed="$failed $base"
    fi
done
echo "compiled $ok ok, $fail failed"
[ "$fail" -gt 0 ] && printf 'failed:%s\n' "$failed" && printf 'logs in %s\n' "$OUT"
[ "$STRICT" = 1 ] && [ "$fail" -gt 0 ] && exit 1
exit 0
