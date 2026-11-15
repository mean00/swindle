#include "ch32v3x.h"

/*
 */
bool ch32v3x_erase(uint32_t addr, size_t len)
{
    bool ok = true;
    uint32_t cur_addr = addr;
    uint32_t end_addr = cur_addr + len;
    while (cur_addr < end_addr)
    {
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_CH32_FASTERASE);
        WRITE__REG(->t, ADDR, cur_addr);
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_START);
        ch32v3x_wait_not_busy();
        ch32v3x_ctl_clear(CH32V3XX_FMC_CTL_CH32_FASTERASE);

        uint32_t stat = READ__REG(t, STATR);
        if (stat & (CH32V3XX_FMC_STAT_PG_ERR + CH32V3XX_FMC_STAT_WP_ERR))
        {
            ch32v3x_stat_set(CH32V3XX_FMC_STAT_PG_ERR + CH32V3XX_FMC_STAT_WP_ERR);
            ok = false;
            break;
        }
        cur_addr += 256;
    }
    if (ok)
    {
        riscv_stub_exit(1);
    }
    else
    {
        riscv_stub_exit(0);
    }
    return ok;
}

//
