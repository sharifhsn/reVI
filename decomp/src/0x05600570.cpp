/* CRT startup-side initialiser for the stack cookie and the exception table.
 * Takes a pointer whose value and address are published to globals, loads two
 * imported routine pointers, and runs two initterm-style calls. */
#include <stdint.h>
#include <intrin.h>

extern "C" {
int __stdcall IsProcessorFeaturePresent(unsigned feature);

extern uintptr_t g_1808af0;   /* 0x1808af0 */
extern uintptr_t g_1808be8;
extern uintptr_t g_1808b88;
extern uintptr_t g_1808a60;
extern uintptr_t g_1808b70;
extern uint32_t g_1808a50;
extern uint32_t g_1808a54;
extern uint32_t g_1808a68;
extern uintptr_t g_1808a70;
extern uintptr_t g_11e1ac8;
extern uintptr_t g_11e1ac0;
extern uintptr_t g_9431d8;

void sub_56006b4(uintptr_t *);
void sub_5600728(uintptr_t *);

void sub_5600570(uintptr_t a, uintptr_t, uintptr_t, uintptr_t, uintptr_t e)
{
    if (IsProcessorFeaturePresent(0x17))
        __fastfail(2);

    sub_56006b4(&g_1808af0);
    g_1808be8 = a;
    g_1808b88 = (uintptr_t)&a + 8;
    g_1808a60 = g_1808be8;
    g_1808b70 = e;
    g_1808a50 = 0xC0000409;
    g_1808a54 = 1;
    g_1808a68 = 1;
    g_1808a70 = 2;

    volatile uintptr_t tbl[2] = {g_11e1ac8, g_11e1ac0};
    (void)tbl[0];
    sub_5600728(&g_9431d8);
}
}
