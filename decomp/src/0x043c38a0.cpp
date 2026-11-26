/* One-shot subrender pool initialiser: five named worker threads, events and
 * records, then the queue ring. Failures shrink the slot count instead of
 * trapping, except at slot 0. */
#include <stdint.h>

struct slot_rec {
    void *thread;      /* +0x00 */
    void *ev_a;        /* +0x08 */
    void *ev_b;        /* +0x10 */
    char name[0x10];   /* +0x18 */
    void *ctx;         /* +0x28 */
    void *attrs;       /* +0x30 */
    uint32_t in_use;   /* +0x38 */
    uint32_t pad;
};

extern "C" void sub_1400b8000(void *dst, const char *fmt, unsigned n);
extern "C" void *sub_142474930(void);
extern "C" void *sub_1459189e0(void *attrs);
extern "C" void *sub_1419ecd00(void);                 /* CreateEventA(0,0,0,0) */
extern "C" void *sub_144fc21d0(void *name, unsigned flags);
extern "C" void *sub_141ad0fd0(void (*start)(unsigned), unsigned param, unsigned stack,
                              unsigned flags, unsigned prio, void *ctx, unsigned detached);
extern "C" void sub_43c4140(unsigned slot);

static bool pool_up;
static slot_rec pool[5];         /* 0x149c3d670 */

static void trap(void) { __debugbreak(); }

void sub_43c38a0(void *owner)
{
    if (!pool_up) {
        unsigned slots = 5;
        for (unsigned i = 0; i < slots; ++i) {
            sub_1400b8000(pool[i].name, "Subrender:%02u", i);
            pool[i].ctx = sub_142474930();
            if (!pool[i].ctx) {
                if (!i) trap();
                slots = i;
                break;
            }
            pool[i].attrs = sub_1459189e0(&pool[i].ctx);
            pool[i].in_use = 1;
            pool[i].ev_a = sub_1419ecd00();
            pool[i].ev_b = sub_1419ecd00();
            pool[i].thread = sub_141ad0fd0(sub_43c4140, i, 0x57800, 0, 0x1f00,
                                           sub_144fc21d0(pool[i].name, 0), 0);
        }
        for (unsigned off = 0x18; off != 0x1818; off += 0x30)
            *(void **)((char *)0x149c3be60 + off) = sub_1419ecd00();
        pool_up = true;
    }
    (void)owner;
}
