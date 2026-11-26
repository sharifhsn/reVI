#!/bin/sh
# Proof that a swapped body runs our C, not the original bytes.
#
#   tools/oracle/swap.sh EXE REVI OBJDIR RVA ARG1 [ARG2...]
#
# Corrupts the extent in a copy of the image, shows the original now crashes,
# patches our compiled object over it and requires the same low byte of rax as
# the pristine image. Narrow return types leave the upper bits dead, so only the
# low byte is compared.
set -u
EXE=${1:?exe}; REVI=${2:?revi}; OBJ=${3:?objdir}; RVA=${4:?rva}; shift 4
NAME=$(printf '0x%08x' "$RVA")

T=$(mktemp -d)
rc=0

FILE_OFF=$((RVA - 0x1000 + 0x400))

cp "$EXE" "$T/corrupt.exe"
python3 - "$T/corrupt.exe" "$FILE_OFF" "$OBJ/$NAME.obj" <<'PY'
import struct, sys
p, off, obj = sys.argv[1], int(sys.argv[2]), sys.argv[3]
d = bytearray(open(p, 'rb').read())
n = len(open(obj, 'rb').read())
# extent length from the object's .text size
o = open(obj, 'rb').read()
nsec = struct.unpack_from('<H', o, 2)[0]
sz = 0
for i in range(nsec):
    at = 20 + 40 * i
    if o[at:at + 5] == b'.text':
        sz = struct.unpack_from('<I', o, at + 16)[0]
d[off:off + sz] = b'\xcc' * sz
open(p, 'wb').write(d)
PY

args=""
for a in "$@"; do args="$args --arg $a"; done

if REVI_STUBS=1 $REVI call "$T/corrupt.exe" "$(printf '0x%x' "$RVA")" $args >/dev/null 2>/dev/null; then
    echo "FAIL: corrupted extent still ran ($NAME)"
    rc=1
else
    echo "ok: corrupted extent dies"
fi

want=$(REVI_STUBS=1 $REVI call "$EXE" "$(printf '0x%x' "$RVA")" $args 2>/dev/null | grep -o 'rax 0x[0-9a-f]*' | cut -d' ' -f2)
got=$(REVI_STUBS=1 $REVI call "$T/corrupt.exe" "$(printf '0x%x' "$RVA")" $args --obj "$OBJ/$NAME.obj" 2>/dev/null | grep -o 'rax 0x[0-9a-f]*' | cut -d' ' -f2)
if [ -z "$got" ]; then
    echo "FAIL: swapped body did not return"
    rc=1
elif python3 -c "import sys; sys.exit(int(sys.argv[1],16)&0xff != int(sys.argv[2],16)&0xff)" "$want" "$got"; then
    echo "ok: swapped body matches pristine (${got%??}xx)"
else
    echo "FAIL: swapped body $got vs pristine $want"
    rc=1
fi

rm -rf "$T"
exit "$rc"
