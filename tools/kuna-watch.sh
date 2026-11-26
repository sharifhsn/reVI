#!/bin/sh
# Watch kuna for fixes (upstream merge base + our fork) and verify them.
#
#   tools/kuna-watch.sh [minutes]
#
# Upstream movement: fetch, merge into the branch, build, run the repro verdicts,
# push to the fork when the build and the regression block both pass. A merge
# conflict stops the iteration and prints the files for hand resolution.
# One line per poll otherwise. Never closes an issue.
set -u
UP=Noelo-Lab/kuna
FORK=sharifhsn/kuna
MIN=${1:-30}
HOST=${HOST:-aster}
BASE_UP=$(timeout 30 ssh "$HOST" 'git -C ~/kuna-fork rev-parse upstream/main' 2>/dev/null)
echo "baseline up=$BASE_UP, watching for $MIN min"

i=0
while [ "$i" -lt "$MIN" ]; do
    sleep 60
    i=$((i + 1))
    NEW=$(timeout 40 ssh "$HOST" "git -C ~/kuna-fork ls-remote https://github.com/$UP refs/heads/main" 2>/dev/null | cut -f1)
    if [ -z "$NEW" ]; then echo "$(date +%H:%M) no answer"; continue; fi
    if [ "$NEW" = "$BASE_UP" ]; then echo "$(date +%H:%M) unchanged"; continue; fi

    echo "$(date +%H:%M) upstream moved $BASE_UP -> $NEW"
    BASE_UP=$NEW
    out=$(timeout 120 ssh "$HOST" 'cd ~/kuna-fork && git fetch -q upstream main && git merge --no-edit upstream/main' 2>&1)
    if echo "$out" | grep -q CONFLICT; then
        echo "  CONFLICT: $(echo "$out" | grep '^CONFLICT' | sed 's/.*: //' | tr '\n' ' ')"
        continue
    fi
    echo "  building"
    timeout 600 ssh "$HOST" 'export PATH=$HOME/.cargo/bin:$PATH; cd ~/kuna-fork/decompiler && nice -n 10 cargo build -j2 --release -p kuna-cli >~/build-watch.log 2>&1; echo build-rc=$?'
    if ! timeout 30 ssh "$HOST" 'test -x ~/kuna-fork/decompiler/target/release/kuna && ~/kuna-fork/decompiler/target/release/kuna --version' >/dev/null 2>&1; then
        echo "  BUILD FAILED, see ~/build-watch.log"; continue
    fi
    echo "  verdicts:"
    v=$(timeout 400 ssh "$HOST" 'bash ~/revi-tools/kuna-repro.sh' 2>&1)
    echo "$v" | grep -E "PASS|PARTIAL|FAIL|REGRESSION|rows" | sed 's/^/    /'
    if echo "$v" | grep -q "REGRESSION FAIL"; then
        echo "  regression failed, not pushed"; continue
    fi
    TOK=$(gh auth token 2>/dev/null)
    if [ -n "$TOK" ]; then
        timeout 120 ssh "$HOST" "cd ~/kuna-fork && git push -q https://x-access-token:$TOK@github.com/$FORK HEAD:main" && echo "  pushed to fork"
    fi
done
