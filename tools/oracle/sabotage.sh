#!/bin/sh
# Negative tests for the vector gate: a mutated expectation must fail replay.
#
#   tools/oracle/sabotage.sh EXE REVI
set -u
EXE=${1:?exe}
REVI=${2:?revi}
HERE=$(dirname "$0")
T=$(mktemp -d)
rc=0

python3 - "$HERE/examples/writes.jsonl" "$T/badhash.jsonl" <<'PY'
import json, sys
v = json.loads(open(sys.argv[1]).read().splitlines()[0])
v["writes"][0]["sha256"] = "0" * 64
open(sys.argv[2], "w").write(json.dumps(v) + "\n")
PY
python3 "$HERE/replay.py" "$T/badhash.jsonl" --exe "$EXE" --revi "$REVI" >/dev/null 2>&1 \
    && { echo "FAIL: wrong write hash accepted"; rc=1; } || echo "ok: wrong write hash rejected"

python3 - "$HERE/examples/writes.jsonl" "$T/badret.jsonl" <<'PY'
import json, sys
v = json.loads(open(sys.argv[1]).read().splitlines()[0])
v["returns"] = "0xdead"
open(sys.argv[2], "w").write(json.dumps(v) + "\n")
PY
python3 "$HERE/replay.py" "$T/badret.jsonl" --exe "$EXE" --revi "$REVI" >/dev/null 2>&1 \
    && { echo "FAIL: wrong return accepted"; rc=1; } || echo "ok: wrong return rejected"

python3 "$HERE/replay.py" "$HERE/examples/writes.jsonl" --exe "$EXE" --revi "$REVI" >/dev/null 2>&1 \
    || { echo "FAIL: true vector rejected"; rc=1; }

rm -rf "$T"
exit "$rc"
