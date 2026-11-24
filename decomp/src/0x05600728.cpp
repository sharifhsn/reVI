/* Last-chance filter: drop the per-process handler, run the default filter on
 * this exception info, then terminate the process with STATUS_STACK_BUFFER_OVERRUN. */
#include <stdint.h>
#include <intrin.h>

extern "C" {
void *__stdcall SetUnhandledExceptionFilter(void *filter);
long __stdcall UnhandledExceptionFilter(void *info);
void *__stdcall GetCurrentProcess(void);
int __stdcall TerminateProcess(void *process, unsigned code);

void sub_5600728(void *info)
{
    SetUnhandledExceptionFilter(0);
    UnhandledExceptionFilter(info);
    TerminateProcess(GetCurrentProcess(), 0xC0000409);
}
}
