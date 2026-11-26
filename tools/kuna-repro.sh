#!/bin/bash
# Re-run every repro we have filed upstream and print one verdict per issue.
# Exits 0 always; the verdicts are the output.
K=${K:-$HOME/kuna-fork/decompiler/target/release/kuna}
E=${E:-$HOME/leonida/drop/exe/game_scarlett_master.exe}
S=${S:-$HOME/kuna-specs/specs}
D() { $K decompile "$E" "$1" --addr --sleighpath "$S" 2>&1; }

echo "== 1968 rotate + returned value"
o=$(D 0x1455ea040)
if grep -qE "ror|rol" <<<"$o"; then
    grep -qE "^void" <<<"$o" && echo "1968 PARTIAL: rotate present, still void" || echo "1968 PASS"
else
    echo "1968 FAIL"; echo "$o" | head -5 | sed 's/^/   /'
fi

echo "== 1969 float half + index parameter"
o=$(D 0x140b4a090)
if grep -qE "^float" <<<"$o" && grep -qE "a1|int .*i" <<<"$o"; then echo "1969 PASS"
elif grep -qE "^float" <<<"$o"; then echo "1969 PARTIAL: float type, parameter still missing"
else echo "1969 FAIL"; echo "$o" | head -4 | sed 's/^/   /'; fi

echo "== 1970 vftable without RTTI"
o=$(D 0x141a34480)
if grep -qiE "vftable|pVftable|slot" <<<"$o"; then echo "1970 PASS"
else echo "1970 FAIL"; echo "$o" | head -4 | sed 's/^/   /'; fi

echo "== 1972 outlining"
$K catalog 2>/dev/null | grep -qE "^outline(shape|detect|auto)" && echo "1972 new detector option present" || echo "1972 no detector option yet"

echo "== regression: boundaries + export + query"
head -4 /tmp/sel400.txt > /tmp/sel4b.txt 2>/dev/null || exit 0
A=$(sed 's/^/--addr /' /tmp/sel4b.txt | tr '\n' ' ')
rm -rf /tmp/kr
$K decompile-project "$E" -o /tmp/kr --layout turso --only-declared --no-asm \
  --define-function @/tmp/bounds.txt --sleighpath "$S" $A >/tmp/kr.log 2>&1 || { echo "REGRESSION FAIL (export)"; tail -3 /tmp/kr.log; exit 0; }
n=$(find /tmp/kr -name '*.c' | wc -l)
c=$(grep -c '"functions"' /tmp/kr/index.jsonl 2>/dev/null || echo 0)
echo "export ok, index rows $(wc -l < /tmp/kr/index.jsonl)"
$K query /tmp/kr/functions.db --range 0x0..0xffffffff --json 2>/dev/null \
  | python3 -c 'import json,sys;d=json.load(sys.stdin);print("query ok, rows",d["counts"]["functions"])' || echo "REGRESSION FAIL (query)"
