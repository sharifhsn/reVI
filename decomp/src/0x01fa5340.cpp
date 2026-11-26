/* Alias of 0x01fa5270: call slot 6 of the global's vftable with the global as
 * `this`, then return true. Same body, different RIP displacement. */
#include <stdint.h>

struct obj { bool (**vt)(obj *); /* ... */ };

extern unsigned char g_17a56410; /* 0x17a56410: object whose slot 6 is called */

bool sub_01fa5340(void)
{
    obj *o = (obj *)&g_17a56410;
    o->vt[6](o);
    return true;
}
