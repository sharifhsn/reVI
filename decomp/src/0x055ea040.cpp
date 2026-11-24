/* Sentinel test around a 16-bit rotation, then a tail call.
 * The sentinel is __security_cookie (0x67e2038). Returns nothing: the result
 * is left in rcx. */
#include <stdint.h>
#include <intrin.h>

extern uintptr_t g_cookie; /* 0x67e2038, __security_cookie */
void sub_5600570(uintptr_t);

void sub_55ea040(uintptr_t v)
{
    if (v == g_cookie) {
        v = _rotl64(v, 16);
        if ((v & 0xffff) == 0)
            return;
        v = _rotr64(v, 16);
    }
    sub_5600570(v);
}
