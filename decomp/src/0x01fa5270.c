/* Instruction-identical to the original, displacements aside. */
#include <stdbool.h>

extern char *g; /* 0xaa56540 */

bool sub_1fa5270(void)
{
    (*(void (**)(void *))(g + 0x30))(&g);
    return true;
}
