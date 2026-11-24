/* Fills the four slot-handler entries. The factory takes a selector; all four
 * calls here pass 0, so the table starts with four handles to one kind. */
#include <stdint.h>

typedef uintptr_t slot_fn;

extern slot_fn g_slots[4]; /* 0x75defc0 */
extern "C" slot_fn sub_1662430(unsigned selector);

void sub_5cabc40(void)
{
    g_slots[0] = sub_1662430(0);
    g_slots[1] = sub_1662430(0);
    g_slots[2] = sub_1662430(0);
    g_slots[3] = sub_1662430(0);
}
