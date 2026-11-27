/* Draft: payload_push/pop and tls_set are sketches of the inline ring and TLS
 * stores, not separate functions. Runs one render job: install it in TLS, begin the GPU pass(es), record them in
 * the payload ring, commit, invoke the job, release. Payloads may be absent. */
#include <stdint.h>
#include <intrin.h>

struct job { void *vt[8]; uint32_t pass; };
struct payload;
struct job_ctx;

static inline void payload_push(payload *p, unsigned char pass);
static inline void payload_pop(payload *p);

extern "C" void sub_4fa85e0(void);
extern "C" void sub_4fa9340(void);
extern "C" void sub_4fa8650(int);
extern "C" void sub_55c11d0(payload *);
extern "C" void sub_55c11f0(payload *);
extern "C" void sub_4e8a350(payload *, void *);
extern "C" void sub_4e89c50(payload *, void *);
extern "C" int sub_55cc020(uint64_t buf, uint32_t pass, unsigned which);
extern "C" void sub_55cc0c0(void);
extern "C" void sub_1443c3890(job *);
extern "C" void sub_4fb3ab0(payload *);
extern "C" void *sub_4da9e70(payload *, unsigned);
extern "C" void sub_4ea5b20(void *gpu, job *);
extern "C" void sub_4deb5a0(job_ctx *);
extern "C" void sub_4e37260(payload *);

extern uint64_t g_gpu;              /* 0x149c3b740 */
extern bool g_hook_a, g_hook_b;     /* 0x149c3b788, 0x149c3b790 */
extern bool g_after_a, g_after_b;   /* 0x149c3b798, 0x149c3b7a0 */

struct job_ctx {
    job *item;                      /* +0x00 */
    /* ... */
    void *slot;                     /* +0x20 */
    /* ... */
    void *result;                   /* +0x10 */
};

static inline void tls_set(job *j, payload *a, payload *b);

void sub_43c4370(job_ctx *ctx, payload *a, payload *b)
{
    tls_set(ctx->item, a, b);
    if (a) { sub_4fa85e0(); sub_4fa8650(-1); }
    if (b) { sub_4fa9340(); sub_4fa8650(-1); }
    if (g_hook_a) sub_55c11d0(a);
    if (g_hook_b) sub_55c11f0(b);

    sub_4e8a350(a, &ctx->slot);
    if (b) sub_4e89c50(b, &ctx->slot);

    unsigned char p1 = (unsigned char)sub_55cc020(*(uint64_t *)(g_gpu + 0x500), ctx->item->pass, 0);
    if (p1 != 0xff) payload_push(a, p1);
    unsigned char p2 = 0xff;
    if (b) {
        p2 = (unsigned char)sub_55cc020(*(uint64_t *)(g_gpu + 0x500), ctx->item->pass, 1);
        if (p2 != 0xff) payload_push(b, p2);
    }
    sub_55cc0c0();
    sub_1443c3890(ctx->item);
    if (p1 != 0xff) payload_pop(a);
    if (b && p2 != 0xff) payload_pop(b);
    if (b) sub_4fb3ab0(b);

    ctx->result = sub_4da9e70(a, 0);
    sub_4ea5b20((void *)g_gpu, ctx->item);
    sub_4deb5a0(ctx);
    if (g_after_a) sub_4e37260(a);
    if (g_after_b) sub_4e37260(b);
    tls_set(nullptr, nullptr, nullptr);
}

/* Sketches of the inline ring in the original: depth at +0x12294, handles from
 * +0x12290. Kept as helpers only so the body reads. */
static inline void payload_push(payload *p, unsigned char pass)
{
    unsigned char *c = (unsigned char *)p + 0x12294;
    *c += 1;
    *(unsigned char *)((char *)p + 0x12290 + *c) = (char)pass;
}

static inline void payload_pop(payload *p)
{
    *(unsigned char *)((char *)p + 0x12294) -= 1;
}

/* TLS stores: job at +0x10e8, a at +0x12a8, b at +0x1108, via gs:0x58. The
 * original re-materialises the pointer per use; one helper is enough here. */
static inline char *tls_vec(void)
{
    return (char *)__readgsqword(0x58);
}

static inline void tls_set(job *j, payload *a, payload *b)
{
    char *t = tls_vec();
    *(job **)(t + 0x10e8) = j;
    *(payload **)(t + 0x12a8) = a;
    *(payload **)(t + 0x1108) = b;
}
