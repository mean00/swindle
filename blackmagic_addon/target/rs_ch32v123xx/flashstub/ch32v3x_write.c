#include "ch32v3x.h"

/*
 */
bool ch32v3x_write(uint32_t dest, uint32_t adr_source, size_t len)
{
    uint32_t ret = 0;
    uint32_t cur_addr = dest;
    const uint8_t *src = (const uint8_t *)adr_source;
    uint32_t end_addr = cur_addr + len;
    while (cur_addr < end_addr)
    {
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_CH32_FASTPROGRAM);
        ch32v3x_wait_not_busy();
        // prefill write cache, we write 256 bytes at a time
        for (int i = 0; i < 64; i++)
        {
            uint32_t data32 = (src[0]) + (src[1] << 8) + (src[2] << 16) + (src[3] << 24);
            target_mem_write32(flash->t, cur_addr, data32);
            src += 4;
            cur_addr += 4;
            ch32v3x_wait_not_wr_busy();
        }
        // and flush
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_CH32_FASTSTART); // and go
        ch32v3x_wait_not_busy();

        ch32v3x_ctl_clear(CH32V3XX_FMC_CTL_PG); // done
        uint32_t stat = READ__REG(t, STATR);
        if (stat & (CH32V3XX_FMC_STAT_PG_ERR + CH32V3XX_FMC_STAT_WP_ERR))
        {
            ch32v3x_stat_set(CH32V3XX_FMC_STAT_PG_ERR + CH32V3XX_FMC_STAT_WP_ERR); // clear error
            goto qnext;
        }
        ch32v3x_stat_set(CH32V3XX_FMC_STAT_WP_ENDF); // done tODO TODO
        if (!(stat & CH32V3XX_FMC_STAT_WP_ENDF))
        {
            goto qnext;
        }
    }
    ret = true;
qnext:
    if (ret)
    {
        riscv_stub_exit(0);
    }
    else
    {
        riscv_stub_exit(1);
    }
    return ret;
}

//
