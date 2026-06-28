
/**
 * @file lnbmp_parity.h
 * @brief SWD parity calculation helpers
 */

#pragma once

/**
 * @brief Compute odd parity for a 32-bit value (used in SWD protocol).
 *
 * Reduces the input via XOR-folding down to 4 bits, then looks up the
 * parity from a 16-bit LUT (bit 0 of 0x6996 at position @p value).
 *
 * @param value 32-bit input value.
 * @return 1 if the number of set bits is odd, 0 otherwise.
 */
static inline uint32_t lnOddParity(uint32_t value)
{
    value ^= value >> 16;
    value ^= value >> 8;
    value ^= value >> 4;
    value &= 0xf;
    return (0x6996 >> value) & 1;
}
