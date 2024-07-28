
#pragma once

/**
 */
static inline uint32_t lnOddParity(uint32_t value)
{
    value ^= value >> 16;
    value ^= value >> 8;
    value ^= value >> 4;
    value &= 0xf;
    return (0x6996 >> value) & 1;
}
