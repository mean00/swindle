#pragma once
#include <stdint.h>

#define RP2350_DMA_BASE 0x50000000UL

// Channel registers (stride 0x40 per channel, 4 registers per alias)
typedef struct {
    volatile uint32_t read_addr;
    volatile uint32_t write_addr;
    volatile uint32_t trans_count;
    volatile uint32_t ctrl_trig;
} dma_channel_hw_t;

// Sniffer and global control registers at 0x50000450
typedef struct {
    volatile uint32_t multi_chan_trigger; // 0x450
    volatile uint32_t sniff_ctrl;          // 0x454
    volatile uint32_t sniff_data;          // 0x458
} dma_sniff_hw_t;

#define DMA_CH(ch) ((dma_channel_hw_t *)(RP2350_DMA_BASE + ((uint32_t)(ch) * 0x40U)))
#define DMA_SNIFF  ((dma_sniff_hw_t *)(RP2350_DMA_BASE + 0x450U))

// Channel CTRL_TRIG bitfields (RP2350 Section 12.6.10, Table 1148)
#define DMA_CH_CTRL_EN           (1U << 0)
#define DMA_CH_CTRL_HIGH_PRIO    (1U << 1)
#define DMA_CH_CTRL_DATA_SIZE_8  (0U << 2)
#define DMA_CH_CTRL_INCR_READ    (1U << 4)
#define DMA_CH_CTRL_CHAIN_TO(ch) ((uint32_t)(ch) << 13)
#define DMA_CH_CTRL_TREQ_UNPACED (0x3fU << 17)
#define DMA_CH_CTRL_SNIFF_EN     (1U << 25)
#define DMA_CH_CTRL_BUSY         (1U << 26)

// Sniffer CTRL bitfields (RP2350 Section 12.6.10, Table 1176)
#define DMA_SNIFF_CTRL_EN         (1U << 0)
#define DMA_SNIFF_CTRL_DMACH(ch)  ((uint32_t)(ch) << 1)
#define DMA_SNIFF_CTRL_CALC_CRC32 (0x0U << 5)
#define DMA_SNIFF_CTRL_OUT_INV    (1U << 11)
