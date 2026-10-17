/**
 * @file bmp_helpers.cpp
 * @brief Standalone implementations of small Blackmagic utility functions.
 *
 * Replaces the following files from the blackmagic source tree:
 *   - blackmagic/src/exception.c
 *   - blackmagic/src/maths_utils.c
 *   - blackmagic/src/hex_utils.c
 *   - blackmagic/src/target/stm32_common.c
 *
 * (Note: blackmagic/src/target/gdb_reg.c was completely unused dead code
 *  because Swindle generates all target XML in Rust).
 *
 * This allows removing blackmagic/src/ completely from compilation.
 */

#include <cstdint>
#include <cstddef>
#include <cstdlib>
#include <cstdio>
#include <csetjmp>

extern "C"
{
#include "general.h"
#include "exception.h"
#include "maths_utils.h"
#include "hex_utils.h"
#include "target.h"
#include "stm32_common.h"

/* --------------------------------------------------------------------------
 * exception.c replacement
 * -------------------------------------------------------------------------- */
exception_s *innermost_exception = nullptr;

void raise_exception(const uint32_t type, const char *const msg)
{
    for (exception_s *exception = innermost_exception; exception; exception = exception->outer)
    {
        if (exception->mask & type)
        {
            exception->type = type;
            exception->msg = msg;
            innermost_exception = exception->outer;
            longjmp(exception->jmpbuf, (int)type);
        }
    }
    abort();
}

/* --------------------------------------------------------------------------
 * maths_utils.c replacement
 * -------------------------------------------------------------------------- */
uint8_t ulog2(uint32_t value)
{
    if (!value)
        return UINT8_MAX;
    return (uint8_t)(32U - (uint8_t)__builtin_clz(value));
}

uint8_t calculate_odd_parity(const uint32_t value)
{
    return (uint8_t)__builtin_parity(value);
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

bool read_unum32(const char *input, const char **rest, uint32_t *val, char follow, int base)
{
    char *end = nullptr;
    uint32_t result = (uint32_t)strtoul(input, &end, base);
    if (!end || end == input)
        return false;
    if (follow != READ_HEX_NO_FOLLOW)
    {
        if (*end == follow)
            end++;
        else
            return false;
    }
    if (rest)
        *rest = end;
    if (val)
        *val = result;
    return true;
}



/* --------------------------------------------------------------------------
 * stm32_common.c replacement
 * -------------------------------------------------------------------------- */
bool stm32_uid(target_s *const target, const target_addr_t uid_base)
{
    uint8_t uid_bytes[12] = {0};
    if (target_mem32_read(target, uid_bytes, uid_base, 12))
        return false;
    char uid_hex[25] = {0};
    for (size_t i = 0; i < 12U; i += 4U)
    {
        uint32_t value = (uint32_t)uid_bytes[i] | ((uint32_t)uid_bytes[i + 1] << 8) |
                         ((uint32_t)uid_bytes[i + 2] << 16) | ((uint32_t)uid_bytes[i + 3] << 24);
        snprintf(uid_hex + i * 2U, 9, "%08X", (unsigned int)value);
    }
    tc_printf(target, "0x%s\n", uid_hex);
    return true;
}

} // extern "C"
