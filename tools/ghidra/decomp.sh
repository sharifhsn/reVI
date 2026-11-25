#!/bin/sh
# Ghidra headless: import+analyse once (keeps the project), then decompile with
# the project already warm. Output JSONL. Heap is raised; the default 2 GB is
# not enough for a 120 MB image.
set -eu
MODE=${1:?usage: decomp.sh import|run <exe> <out.jsonl> [rva-list]}
shift
export GHIDRA_HEADLESS_MAXMEM=${GHIDRA_HEADLESS_MAXMEM:-12G}
GHIDRA=${GHIDRA:-$HOME/tools/ghidra_12.2_PUBLIC/support/analyzeHeadless}
PROJ=${PROJ:-$HOME/ghidra-proj}
HERE=$(dirname "$0")
case "$MODE" in
  import)
    EXE=${1:?exe}
    "$GHIDRA" "$PROJ" revi -import "$EXE" -overwrite -scriptPath "$HERE" 2>&1 | tail -5
    ;;
  run)
    OUT=${1:?out}
    LIST=${2:-}
    if [ -n "$LIST" ]; then
      "$GHIDRA" "$PROJ" revi -process -noanalysis -scriptPath "$HERE" \
        -postScript ExportDecomp.java "$OUT" "$LIST" 2>&1 | tail -5
    else
      "$GHIDRA" "$PROJ" revi -process -noanalysis -scriptPath "$HERE" \
        -postScript ExportDecomp.java "$OUT" 2>&1 | tail -5
    fi
    ;;
esac
