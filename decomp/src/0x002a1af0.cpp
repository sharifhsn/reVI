/* Looks a record up under the object's read lock and answers whether its value
 * is above the record's own ceiling or below a tiny floor. The lock pair and the
 * lookup are the interesting parts; slots 0x38/0x40/0x58 are positional. */
#include <stdint.h>

struct rec;

struct rec_vtbl {
    void *gap0[7];                 /* 0x00..0x30 */
    float (*get_a)(rec *);
    float (*get_b)(rec *);
    void *gap1[2];
    bool (*pred)(rec *);           /* 0x58 */
};

struct rec {
    const rec_vtbl *vtbl;
    /* ... */
};

struct rwlock {
    volatile uint32_t readers;     /* bits 0x1ff0000 */
    void *writer_wait;             /* +0x8 */
    /* ... */
};

struct obj {
    uint32_t tag;                  /* +0x4 */
    uint32_t pad;
    uint64_t id;                   /* +0x8 */
    /* ... */
    rwlock lock;                   /* +0x30 */
};

extern "C" void sub_347fd0(void **guard, rwlock *l);
extern "C" void sub_2c59f0(void *guard);
extern "C" rec *sub_358990(uint32_t tag, uint64_t id, unsigned a1, unsigned a2);

static const float k_floor = 1.3592346e-15f;   /* 0x5fc6f0c */

bool sub_2a1af0(obj *o, unsigned a1, unsigned a2)
{
    void *guard;
    sub_347fd0(&guard, &o->lock);

    bool out = false;
    rec *r = sub_358990(o->tag, o->id, a1, a2);
    if (r && r->vtbl->pred(r)) {
        float a = r->vtbl->get_a(r);
        float b = r->vtbl->get_b(r);
        out = (b < a) | (a < k_floor);
    }

    sub_2c59f0(guard);
    return out;
}
