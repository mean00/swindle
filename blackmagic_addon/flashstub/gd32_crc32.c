#include "stdint.h"
#include "stub.h"
#define CH32_CRC32_ADDR 0x40023000
#define CH32_CRC32_CONTROL_RESET 1
#define AHBPCENR (*(uint32_t *)0x40021014UL)
typedef struct
{
    uint32_t data;
    uint32_t independant_data;
    uint32_t control;
} CRC_IPx;

typedef volatile CRC_IPx CRC_IP;

#if 0
static uint32_t  complete_crc32(uint8_t *data, uint32_t len, uint32_t crc) 
{
   for(int i=0;i<len;i++)
   {
      uint32_t next = data[i];
      crc = crc ^ next;
      for (int j = 7; j >= 0; j--) {
         uint32_t mask = -(crc & 1);
         crc = (crc >> 1) ^ (0xEDB88320 & mask);
      }      
   }
   return ~crc;
}
#endif
/**
 *  crc is in A2
 *  __attribute__((naked))
 */
void __attribute__((naked)) __attribute__((noreturn)) ch32_cr(uint32_t addr, uint32_t len_in_u32, uint32_t *output)
{
    // Enable  CRC clock
    AHBPCENR |= 1 << 6;
    //
    CRC_IP *crc = (CRC_IP *)CH32_CRC32_ADDR;
    // reset writes 0xFFFFFFF by itself
    crc->control = CH32_CRC32_CONTROL_RESET;
    // crc->data = init;
    uint32_t *mem = (uint32_t *)addr;
    uint32_t *lim = mem + len_in_u32;
    while (mem < lim)
    {
        uint32_t in = *(mem++);
        uint32_t out =
            ((in >> 24) & 0xff) | (((in >> 16) & 0xff) << 8) | (((in >> 8) & 0xff) << 16) | ((in & 0xff) << 24);
        // uint32_t out= __builtin_bswap32(in);
        crc->data = out;
    }
    __asm__("mov r1,%0" ::"r"(crc->data));
    //*output = crc->data;
    stub_exit(0);
    while (1)
    {
    };
}

// EOF
