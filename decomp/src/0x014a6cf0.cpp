/* Writes a 12-byte entry: value, hit count, and an encoded handle for a
 * pointer, or the 0x1ffff "none" marker. Index bound is 0x74. */
#include <stdint.h>

struct chunk_desc {
    uintptr_t base;      /* 0x00 */
    uint32_t stride;     /* 0x08 */
    char pad[0x1c];
    uint32_t *map;       /* 0x28 */
};

struct entry {
    uint32_t value;      /* 0x00 */
    uint32_t count;      /* 0x04 */
    uint32_t handle;     /* 0x08 */
};

extern struct chunk_desc *g_chunks[256]; /* 0x9717ad0 */

void sub_14a6cf0(struct entry *table, unsigned index, unsigned value, void *p,
                 unsigned count)
{
    if (index > 0x74)
        return;

    struct entry *e = &table[index];
    e->value = value;
    e->count++;

    if (p) {
        unsigned chunk = *(unsigned char *)((char *)p + 0x30);
        struct chunk_desc *d = g_chunks[chunk];
        uint32_t offset = (uint32_t)(((uintptr_t)p - d->base) / d->stride) & 0x1ffff;
        uint32_t extra = d->map[offset];
        e->handle = (chunk & 0x1f) << 17 | offset | (extra << 5 & 0xffc00000);
    } else {
        e->handle = 0x1ffff;
    }

    if (count != 0xffffffffu)
        e->count = count;
}
