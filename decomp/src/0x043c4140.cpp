/* Body of one subrender worker thread: claim the slot, then pop and run work
 * until the queue drains. The TLS slot byte written here is what the slot
 * semaphores and the try-reserve are keyed on. */
#include <stdint.h>

struct item;

struct slot_rec {
    void *handle;      /* +0x00 */
    void *pad;         /* +0x08 */
    void *arg_b;       /* +0x18 */
    void *obj;         /* +0x20 */
    /* ... */
    uint32_t flags;    /* +0x30 */
};

extern "C" void sub_144fab4b0(unsigned v);
extern "C" void sub_1455db994(void);
extern "C" void sub_1401784d0(void *handle);            /* WaitForSingleObject veneer */
extern "C" void sub_1402a1be0(void *sem, unsigned n);   /* counted wait */
extern "C" void sub_1443c4370(item *it, void *b, void *a);
extern "C" void sub_140005930(void *p);

static slot_rec *rec(void);

void sub_43c4140(unsigned slot)
{
    sub_144fab4b0(4);
    /* tls->slot = slot; tls->in_use = 1; */
    for (;;) {
        if (!(rec()[slot].flags & 1))
            return;
        sub_1401784d0(rec()[slot].handle);
        /* attention drain on 0x149c3be58 */
        item *it = nullptr; /* ring pop from 0x149c3ba30 */
        if (!it)
            break;
        sub_1443c4370(it, rec()[slot].arg_b, rec()[slot].obj);
        sub_140005930(*(void **)&((void **)it)[3]);
    }
    sub_140005930(rec()[slot].obj);
}
