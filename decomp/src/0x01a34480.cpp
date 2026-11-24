/* Behaviourally equal to the original; see the codegen note. */
struct B;

typedef void (*v8_fn)(struct B *, void **);

struct B_vt {
    void (*slots[8])(void);
    v8_fn v8;
};

extern char *g; /* 0x48e3430 */

void sub_1a34480(struct B *self)
{
    void *p = &g;
    (*(v8_fn *)(*(char **)self + 0x40))(self, &p);
}
