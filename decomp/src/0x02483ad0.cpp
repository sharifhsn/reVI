/* Closes every slot handle. The last one is a tail jump, so the call is shared. */
#include <stdint.h>

extern void *g_slots[5]; /* 0x75defc0 */

extern "C" int __stdcall CloseHandle(void *handle);

void sub_2483ad0(void)
{
    CloseHandle(g_slots[0]);
    CloseHandle(g_slots[1]);
    CloseHandle(g_slots[2]);
    CloseHandle(g_slots[3]);
    CloseHandle(g_slots[4]);
}
