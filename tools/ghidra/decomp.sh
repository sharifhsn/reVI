#!/bin/sh
# Run Ghidra headless analysis and bulk decompile. Analysis dominates the first
# run; the project is kept so later runs only decompile.
set -eu
EXE=${1:?usage: decomp.sh <exe> <out.jsonl> [rva-list.txt]}
OUT=${2:?out path}
LIST=${3:-}
GHIDRA=${GHIDRA:-$HOME/tools/ghidra_12.2_PUBLIC/support/analyzeHeadless}
PROJ=${PROJ:-$HOME/ghidra-proj}
mkdir -p "$PROJ"
if [ -n "$LIST" ]; then
  "$GHIDRA" "$PROJ" revi -import "$EXE" -overwrite -scriptPath "$(dirname "$0")" \
    -postScript ExportDecomp.java "$OUT" "$LIST" -deleteProject
else
  "$GHIDRA" "$PROJ" revi -import "$EXE" -overwrite -scriptPath "$(dirname "$0")" \
    -postScript ExportDecomp.java "$OUT" -deleteProject
fi
