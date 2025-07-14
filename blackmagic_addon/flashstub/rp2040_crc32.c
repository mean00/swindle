#include "stdint.h"
//
#include "rp_dmax.h"

#define dma_chan 0
#include "stub.h"
/*
 * Use RP2040 sniffer to compute CRC
 * output is in r1
 */
void __attribute__((naked)) __attribute__((noreturn))
rp_crc32(uint32_t len, uint32_t address, uint32_t *tmp) {
  // Initialize stdio
  LN_RP_DMA_channel *channel = (LN_RP_DMA_channel *)LN_RP_DMA_CHANNEL_BASE;
  LN_RP_DMA *dma = (LN_RP_DMA *)LN_RP_DMA_CONTROL;
  LN_RP_DMA_EXT *dma_ext = (LN_RP_DMA_EXT *)LN_RP_DMA_EXT_ADDR;

  channel = RP_DMA_CHANNEL(dma_chan);
  // Apply configuration
  uint32_t _control = 0;
  _control |= LN_RP_DMA_CONTROL_SET_DATA_SIZE_8;
  _control |= LN_RP_DMA_CONTROL_INCR_READ;
  _control |= LN_RP_DMA_CONTROL_HIGH_PRIO;
  _control |= LN_RP_DMA_CONTROL_CHAIN_TO(dma_chan); // chain to self
  _control |= LN_RP_DMA_CONTROL_INCR_READ;
  _control |= LN_RP_DMA_CONTROL_SNIFF_ENABLE;
  _control |= LN_RP_DMA_CONTROL_ENABLE;
  _control |= LN_RP_DMA_CONTROL_TREQ(0x3f); // no trigger
  channel->DMA_CONTROL = _control;
  channel->DMA_COUNT = len;
  channel->DMA_READ = address;
  channel->DMA_WRITE = (uint32_t)tmp;
  dma_ext->SNIFF_DATA = 0xffffffffU;
  dma_ext->SNIFF_CTRL = LN_RP_DMAEXT_CALC_CRC32 | LN_RP_DMAEXT_OUTPUT_INV |
                        0 * LN_RP_DMAEXT_OUTPUT_REV | (dma_chan << 1) |
                        LN_RP_DMAEXT_ENABLE;
  dma_ext->MULTI_CHANNEL_TRIGGER = 1 << dma_chan;

  // Wait for transfer to complete
  while (channel->DMA_CONTROL & LN_RP_DMA_CONTROL_IS_BUSY_MASK) {
    __asm__("nop");
  }
  uint32_t out = dma_ext->SNIFF_DATA;
  __asm__("mov r1,%0" ::"r"(out));
  stub_exit(0);
}
#if 0
/*
 *
 *
 *
 */
int main(void) {
  const uint8_t *src_buffer = (const uint8_t *)"123456789";
  // expecting 0xfc891918
  volatile uint32_t r = rp_crc32(9, (uint32_t)src_buffer);
  __asm__("bkpt #0");
  while (1) {
    __asm__("nop");
  }
  return 0;
}
#endif
