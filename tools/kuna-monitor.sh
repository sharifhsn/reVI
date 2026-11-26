#!/bin/sh
# Watch the fork for agent fix branches and our issues for new comments.
#
#   tools/kuna-monitor.sh [minutes]
#
# A new branch is checked out into a throwaway worktree, built with the shared
# target dir, and put through tools/kuna-repro.sh. The branch is never merged and
# never pushed; the log says what it did. New issue comments are echoed so a
# reviewer can act on them.
set -u
FORK=sharifhsn/kuna
ISSUES="1968 1969 1970 1971 1972 1978 1997 1998 1999"
MIN=${1:-120}
HOST=${HOST:-aster}
KNOWN=$(timeout 40 ssh "$HOST" "git -C ~/kuna-fork ls-remote https://github.com/$FORK 'refs/heads/*'" 2>/dev/null | cut -f2 | sed 's#refs/heads/##' | sort)
echo "baseline: $(echo "$KNOWN" | wc -l) branches"
for n in $ISSUES; do
    c=$(gh issue view "$n" -R Noelo-Lab/kuna --json comments --jq '.comments|length' 2>/dev/null || echo 0)
    echo "$n:$c"
done > /tmp/kuna-seen-counts

i=0
while [ "$i" -lt "$MIN" ]; do
    sleep 120
    i=$((i + 2))

    for n in $ISSUES; do
        c=$(gh issue view "$n" -R Noelo-Lab/kuna --json comments --jq '.comments|length' 2>/dev/null || echo 0)
        was=$(grep "^$n:" /tmp/kuna-seen-counts | cut -d: -f2)
        if [ -n "$was" ] && [ "$c" -gt "$was" ] 2>/dev/null; then
            echo "$(date +%H:%M) issue $n: comment $was -> $c"
            gh issue view "$n" -R Noelo-Lab/kuna --json comments --jq ".comments[$was:]|.[]|.author.login+\": \"+(.body[0:200]|gsub(\"\n\";\" \"))" 2>/dev/null | sed 's/^/    /'
        fi
        sed -i.bak "s/^$n:.*/$n:$c/" /tmp/kuna-seen-counts 2>/dev/null || true
    done

    NOW=$(timeout 40 ssh "$HOST" "git -C ~/kuna-fork ls-remote https://github.com/$FORK 'refs/heads/*'" 2>/dev/null | cut -f2 | sed 's#refs/heads/##' | sort)
    [ -z "$NOW" ] && { echo "$(date +%H:%M) no answer"; continue; }
    NEW=$(echo "$NOW" | grep -vxF "$KNOWN" || true)
    KNOWN=$NOW
    [ -z "$NEW" ] && { echo "$(date +%H:%M) quiet"; continue; }
    for b in $NEW; do
        echo "$(date +%H:%M) branch $b"
        timeout 120 ssh "$HOST" "cd ~/kuna-fork && git fetch -q https://github.com/$FORK 'refs/heads/$b:refs/remotes/fix/$b' && git worktree remove --force ~/kuna-verify 2>/dev/null; git worktree add --detach ~/kuna-verify 'refs/remotes/fix/$b'" >/dev/null 2>&1
        if [ $? -ne 0 ]; then echo "  worktree failed"; continue; fi
        timeout 600 ssh "$HOST" 'export PATH=$HOME/.cargo/bin:$PATH CARGO_TARGET_DIR=$HOME/kuna-fork/decompiler/target; cd ~/kuna-verify/decompiler && nice -n 10 cargo build -j2 --release -p kuna-cli >~/build-verify.log 2>&1; echo build-rc=$?'
        v=$(timeout 400 ssh "$HOST" 'K=$HOME/kuna-fork/decompiler/target/release/kuna bash ~/revi-tools/kuna-repro.sh' 2>&1)
        echo "$v" | grep -E "PASS|PARTIAL|FAIL|REGRESSION|rows" | sed 's/^/  /'
        timeout 60 ssh "$HOST" 'git -C ~/kuna-fork worktree remove --force ~/kuna-verify' >/dev/null 2>&1
    done
done
