/* Semaphore factory with the platform OOM recovery: create, and on an OOM
 * failure trim the working set and retry once, then trap. The initial count is
 * the selector, so selector 0 yields an unavailable semaphore. */
#include <stdint.h>

extern "C" void *CreateSemaphoreExW(const void *attrs, int32_t initial, int32_t max,
                                   const uint16_t *name, uint32_t flags, uint32_t access);
extern "C" bool is_oom(bool trap);        /* 0x2214c10 */
extern "C" void trim_working_set(void);   /* 0x2215fd0 */

static inline void *create(unsigned selector)
{
    return CreateSemaphoreExW(nullptr, (int32_t)selector, 0x7fff, nullptr, 0, 0x100002);
}

void *sub_1662430(unsigned selector)
{
    void *h = create(selector);
    if (h)
        return h;
    if (!is_oom(false))
        __debugbreak();
    trim_working_set();
    h = create(selector);
    if (h)
        return h;
    is_oom(false);
    __debugbreak();
}
