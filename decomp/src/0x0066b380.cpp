/* Virtual detach: puts the owned child into state 0, then clears the owner's
 * flag. Only reached through pointer tables (four vtables), so `this` is RCX. */
#include <stdint.h>

struct child;

struct owner {
    /* ... */
    child *child_0x30;
    /* ... */
    bool flag_0x41;
};

struct child_field;

extern "C" child_field *sub_530dc70(child *c);            /* c->field_0x10 */
extern "C" void sub_b844a0(void *obj, unsigned char state); /* state setter */

void sub_66b380(owner *o)
{
    sub_b844a0(sub_530dc70(o->child_0x30), 0);
    o->flag_0x41 = false;
}
