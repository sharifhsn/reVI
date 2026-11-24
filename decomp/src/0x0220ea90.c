/* Instruction-identical to the original, displacements aside. */
#include <stdbool.h>

extern char *g; /* 0xaad5eb0 */

bool sub_220ea90(void)
{
    (*(void (**)(void *))(g + 0x28))(&g);
    return true;
}
