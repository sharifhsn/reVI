#include <stdint.h>
typedef unsigned long(__stdcall *wait_fn)(void *, unsigned);
extern wait_fn g_wait;
void sub_569f0(void *handle, unsigned count)
{
    for (; count != 0; --count)
        g_wait(handle, 0xffffffff);
}
