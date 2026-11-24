/* Captures the current context and walks two frames up with RtlVirtualUnwind,
 * collecting the handler and establisher of each. RIP is at CONTEXT+0xF8. */
#include <stdint.h>

struct CONTEXT {
    char pad[0xf8];      /* 0x000 */
    uint64_t Rip;        /* 0xf8 */
};

struct RUNTIME_FUNCTION;

extern "C" {
void __stdcall RtlCaptureContext(CONTEXT *context);
RUNTIME_FUNCTION *__stdcall RtlLookupFunctionEntry(uint64_t pc, uint64_t *image_base,
                                                   void *history);
void *__stdcall RtlVirtualUnwind(unsigned handler_type, uint64_t image_base, uint64_t pc,
                                 RUNTIME_FUNCTION *entry, CONTEXT *context, void **handler,
                                 void **establisher, void *context_pointers);

void sub_56006b4(CONTEXT *ctx)
{
    RtlCaptureContext(ctx);
    uint64_t pc = ctx->Rip;

    for (unsigned i = 0; i < 2; i++) {
        uint64_t image_base = 0;
        void *context_pointers = 0;
        void *handler;
        void *establisher;
        RUNTIME_FUNCTION *entry = RtlLookupFunctionEntry(pc, &image_base, 0);
        if (entry == 0)
            break;
        RtlVirtualUnwind(0, image_base, pc, entry, ctx, &handler, &establisher,
                         &context_pointers);
    }
}
}
