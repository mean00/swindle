/**
 * @file bmp_functions.cpp
 * @brief Blackmagic utility helpers that only the hosted build needs.
 *
 * These used to live in swindle/src/bmp_helpers.cpp. No probe firmware calls
 * them: their only callers are blackmagic's hosted/remote protocol layer
 * (blackmagic/src/platforms/hosted/**, itself only compiled for this build), so
 * keeping them next to the hosted executables keeps the firmware free of them.
 *
 * Replaces the hosted-useable halves of:
 *   - blackmagic/src/exception.c
 *   - blackmagic/src/hex_utils.c
 */

#include <cstdint>
#include <cstddef>

extern "C"
{
#include "platform_support.h"

    void do_assert(const char *a);

    /* --------------------------------------------------------------------------
     * exception.c replacement
     * -------------------------------------------------------------------------- */
    void raise_exception(const uint32_t type, const char *const msg)
    {
        (void)type;
        (void)msg;
        do_assert("X");
    }

    /* --------------------------------------------------------------------------
     * hex_utils.c replacement
     * -------------------------------------------------------------------------- */
    char hex_digit(const uint8_t value)
    {
        return (value > 9U) ? (char)(value - 10U + 'A') : (char)(value + '0');
    }

    char *hexify(char *const dst, const void *const buf, const size_t size)
    {
        const uint8_t *const src = (const uint8_t *)buf;
        size_t dst_idx = 0U;
        for (size_t src_idx = 0U; src_idx < size; ++src_idx)
        {
            dst[dst_idx++] = hex_digit(src[src_idx] >> 4U);
            dst[dst_idx++] = hex_digit(src[src_idx] & 0x0fU);
        }
        return dst;
    }

    uint8_t unhex_digit(const char hex)
    {
        uint8_t tmp = (uint8_t)(hex - '0');
        if (tmp > 9U)
            tmp -= (uint8_t)('A' - '0' - 10U);
        if (tmp > 16U)
            tmp -= (uint8_t)('a' - 'A');
        return tmp;
    }

    char *unhexify(void *const buf, const char *hex, const size_t size)
    {
        uint8_t *const dst = (uint8_t *)buf;
        for (size_t idx = 0; idx < size; ++idx, hex += 2U)
            dst[idx] = (uint8_t)((unhex_digit(hex[0]) << 4U) | unhex_digit(hex[1]));
        return (char *)buf;
    }

    uint64_t hex_string_to_num(const size_t max_digits, const char *const str)
    {
        uint64_t ret = 0;
        for (size_t offset = 0; offset < max_digits; ++offset)
        {
            const char value = str[offset];
            if (!is_hex(value))
                return ret;
            ret <<= 4U;
            ret |= unhex_digit(value);
        }
        return ret;
    }
}
