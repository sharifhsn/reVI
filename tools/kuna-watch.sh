#!/bin/sh
# Watch the kuna fork for automated fixes to our open issues.
#
#   tools/kuna-watch.sh [minutes]
#
# Prints one line per poll. When main moves it rebuilds on aster and re-runs the
# repros for issues 1968, 1969, 1970 and 1972, then prints PASS/FAIL per issue.
# It never closes an issue; it only reports so a human/agent can decide.
set -eu
REPO=sharifhsn/kuna
MIN=${1:-30}
HOST=${HOST:-aster}
BASE=$(timeout 30 ssh "$HOST" 'git -C ~/kuna-fork rev-parse origin/main')
echo "baseline $BASE, watching $REPO for $MIN min"

i=0
while [ "$i" -lt "$MIN" ]; do
    sleep 60
    i=$((i + 1))
    NEW=$(timeout 30 ssh "$HOST" "git ls-remote https://github.com/$REPO refs/heads/main" 2>/dev/null | cut -f1 || true)
    [ -z "$NEW" ] && { echo "$(date +%H:%M) no answer"; continue; }
    if [ "$NEW" = "$BASE" ]; then
        echo "$(date +%H:%M) unchanged"
        continue
    fi
    echo "$(date +%H:%M) moved $BASE -> $NEW"
    timeout 120 ssh "$HOST" 'cd ~/kuna-fork && git fetch -q origin main && git checkout -q FETCH_HEAD' || { echo "  pull failed"; continue; }
    echo "  rebuilding"
    timeout 400 ssh "$HOST" 'cd ~/kuna-fork/decompiler && PATH=$HOME/.cargo/bin:$PATH cargo build -q --release -p kuna-cli' || { echo "  BUILD FAILED"; continue; }
    echo "--- repros ---"
    timeout 300 ssh "$HOST" 'bash ~/revi-tools/kuna-repro.sh' || true
    BASE=$NEW
done
