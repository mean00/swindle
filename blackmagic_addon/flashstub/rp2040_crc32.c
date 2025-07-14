/*
 *    This is a hw assisted crc32 as use by gdb through compare-sections
 *    The byte order is reversed compared to the native way
 *    ( the flashstub can only process 32 bits aligned data , which is fairly normal)
 *
 *
 */

#include "stdint.h"
#include "stub.h"
#define GD32_CRC32_ADDR 0x40023000
#define GD32_CRC32_CONTROL_RESET 1
#define AHBPCENR (*(uint32_t *)0x40021014UL)
typedef struct
{
    uint32_t data;
    uint32_t independant_data;
    uint32_t control;
} CRC_IPx;

typedef volatile CRC_IPx CRC_IP;

/**
 *  crc output is in r1
 */
void __attribute__((naked)) __attribute__((noreturn)) ch32_cr(uint32_t addr, uint32_t len_in_u32, uint32_t *output)
{
    // Enable  CRC clock
    AHBPCENR |= 1 << 6;
    //
    CRC_IP *crc = (CRC_IP *)GD32_CRC32_ADDR;
    // reset writes 0xFFFFFFF by itself
    crc->control = GD32_CRC32_CONTROL_RESET;
    // crc->data = init;
    uint32_t *mem = (uint32_t *)addr;
    uint32_t *lim = mem + len_in_u32;
    while (mem + 32 < lim)
    {
        crc->data = __builtin_bswap32(mem[0]);
        crc->data = __builtin_bswap32(mem[1]);
        crc->data = __builtin_bswap32(mem[2]);
        crc->data = __builtin_bswap32(mem[3]);
        crc->data = __builtin_bswap32(mem[4]);
        crc->data = __builtin_bswap32(mem[5]);
        crc->data = __builtin_bswap32(mem[6]);
        crc->data = __builtin_bswap32(mem[7]);
        mem += 8;
    }
    while (mem < lim)
    {
        crc->data = __builtin_bswap32(*(mem++));
    }
    uint32_t out = crc->data;
    __asm__("mov r1,%0" ::"r"(out));
    stub_exit(0);
    while (1)
    {
    };
}

// EOF
