/* Fills the five slot-handle entries. The factory takes a selector; all five
 * calls here pass 0, so the table starts with five handles of one kind. */
#include <stdint.h>

typedef uintptr_t slot_handle;

extern slot_handle g_slots[5]; /* 0x75defc0 */
extern "C" slot_handle sub_1662430(unsigned selector);

void sub_5cabc40(void)
{
    g_slots[0] = sub_1662430(0);
    g_slots[1] = sub_1662430(0);
    g_slots[2] = sub_1662430(0);
    g_slots[3] = sub_1662430(0);
    g_slots[4] = sub_1662430(0);
}
