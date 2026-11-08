#include <stdint.h>
#include "rp2350_dma.h"

#define DMA_CHAN 0

int rp2350_crc32(uint32_t len, uint32_t address, uint32_t *tmp, uint32_t *out_crc)
{
    dma_channel_hw_t *ch = DMA_CH(DMA_CHAN);
    dma_sniff_hw_t *sniff = DMA_SNIFF;

    ch->ctrl_trig = 0;

    ch->read_addr = address;
    ch->write_addr = (uint32_t)tmp;
    ch->trans_count = len;

    sniff->sniff_data = 0xffffffffU;
    sniff->sniff_ctrl = DMA_SNIFF_CTRL_CALC_CRC32 |
                        DMA_SNIFF_CTRL_OUT_INV |
                        DMA_SNIFF_CTRL_DMACH(DMA_CHAN) |
                        DMA_SNIFF_CTRL_EN;

    uint32_t ctrl = DMA_CH_CTRL_DATA_SIZE_8 |
                    DMA_CH_CTRL_INCR_READ |
                    DMA_CH_CTRL_HIGH_PRIO |
                    DMA_CH_CTRL_CHAIN_TO(DMA_CHAN) |
                    DMA_CH_CTRL_TREQ_UNPACED |
                    DMA_CH_CTRL_SNIFF_EN |
                    DMA_CH_CTRL_EN;

    ch->ctrl_trig = ctrl;

    while (ch->ctrl_trig & DMA_CH_CTRL_BUSY) {
        if (ch->ctrl_trig & (1U << 31)) { // AHB_ERROR
            return 0;
        }
    }

    if (out_crc) {
        *out_crc = sniff->sniff_data;
    }
    return 1;
}
