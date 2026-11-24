/* Behavioural reconstruction of 0x001117e0. See notes for the original. */
#include <stdbool.h>

/* 0x4f76200, returns in al. */
unsigned char sub_4f76200(int);

bool sub_1117e0(unsigned char a, int b)
{
    if (a == 0x16)
        return false;
    return (sub_4f76200(b) & 0xfe) == 4;
}
