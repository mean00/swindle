#include "riscv32_flashstub.h"
#include "stdint.h"
//
#include "rp_dmax.h"

#define dma_chan 0

/*
 * RISC-V (RP2350 Hazard3) twin of `rp2040_crc32.c`: the same DMA + sniffer CRC32, but
 * the result is published in a2 and the stub exits with `ebreak` (what
 * `riscv32_run_stub()` waits for) instead of returning through r1/`bkpt`.
 *
 * The sniffer register offsets are identical on both parts (DMA at 0x50000000,
 * DMA_CONTROL 0x400, MULTI_CHANNEL_TRIGGER 0x430, SNIFF_CTRL 0x434, SNIFF_DATA 0x438),
 * so this stub serves the RP2350 exactly like the ARM one serves the RP2040.
 *
 * No stack is used: `riscv32_run_stub()` runs the stub on whatever SP the target had.
 */
void rp_crc32_riscv(uint32_t len, uint32_t address, uint32_t *tmp)
{
    LN_RP_DMA_channel *channel = RP_DMA_CHANNEL(dma_chan);
    LN_RP_DMA_EXT *dma_ext = (LN_RP_DMA_EXT *)LN_RP_DMA_EXT_ADDR;

    uint32_t control = 0;
    control |= LN_RP_DMA_CONTROL_SET_DATA_SIZE_8;
    control |= LN_RP_DMA_CONTROL_INCR_READ;
    control |= LN_RP_DMA_CONTROL_HIGH_PRIO;
    control |= LN_RP_DMA_CONTROL_CHAIN_TO(dma_chan); // chain to self
    control |= LN_RP_DMA_CONTROL_SNIFF_ENABLE;
    control |= LN_RP_DMA_CONTROL_ENABLE;
    control |= LN_RP_DMA_CONTROL_TREQ(0x3f); // no trigger
    channel->DMA_CONTROL = control;
    channel->DMA_COUNT = len;
    channel->DMA_READ = address;
    channel->DMA_WRITE = (uint32_t)tmp;
    dma_ext->SNIFF_DATA = 0xffffffffU;
    dma_ext->SNIFF_CTRL = LN_RP_DMAEXT_CALC_CRC32 | LN_RP_DMAEXT_OUTPUT_INV | 0 * LN_RP_DMAEXT_OUTPUT_REV |
                          (dma_chan << 1) | LN_RP_DMAEXT_ENABLE;
    dma_ext->MULTI_CHANNEL_TRIGGER = 1 << dma_chan;

    // Wait for the transfer to complete
    while (channel->DMA_CONTROL & LN_RP_DMA_CONTROL_IS_BUSY_MASK)
    {
        __asm__("nop");
    }
    uint32_t out = dma_ext->SNIFF_DATA;
    __asm__("addi a2,%0,0" ::"r"(out));
    riscv_stub_exit(0);
}

// EOF
