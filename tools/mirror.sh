#!/bin/sh
# Mirror the canonical repository to the public one, so a merge on the private
# host reaches GitHub without anyone remembering to push.
#
# Runs on the host: FORGE is the canonical clone, MIRROR the public remote.
set -eu
FORGE=${FORGE:-/home/sharif/srv/revi/bot-work}
MIRROR=${MIRROR:-https://github.com/sharifhsn/reVI.git}
# A token in the URL keeps the service non-interactive.
case "$MIRROR" in
  https://*) [ -n "${GITHUB_TOKEN:-}" ] && MIRROR="https://x-access-token:${GITHUB_TOKEN}@${MIRROR#https://}" ;;
esac

git -C "$FORGE" fetch -q origin
git -C "$FORGE" push -q --force "$MIRROR" \
    "refs/remotes/origin/main:refs/heads/main"
