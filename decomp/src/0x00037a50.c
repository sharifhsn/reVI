/* Reserves a slot in a packed 32-bit word with a compare-exchange loop, then
 * tail-calls the slot's handler with 1. Returns without reserving on the
 * "already taken" and "older generation" paths. */
#include <stdint.h>
#include <intrin.h>

typedef void (*handler)(unsigned);

extern handler g_slots[256]; /* 0x75a7570 */

void sub_37a50(volatile uint32_t *word, unsigned char flag, unsigned char gen)
{
    void *teb = (void *)__readgsqword(0x58);
    unsigned char slot = *(unsigned char *)(*(char **)teb + 0x1294);

    uint32_t cur = *word;
    uint32_t mine = ((uint32_t)slot << 28) + 0x10000000u;

    if (flag) {
        uint32_t mask = ((uint32_t)gen << 25) | 0xff;
        if (((cur >> 8) & 0xff) < (cur & mask))
            return;
    } else if (cur & (1u << 25)) {
        return;
    }

    for (;;) {
        uint32_t seen = cur;
        uint32_t next = seen | mine;
        uint32_t got = (uint32_t)_InterlockedCompareExchange((volatile long *)word,
                                                             (long)next, (long)seen);
        cur = got;
        if (got == seen)
            break;
    }

    g_slots[slot](1);
}
