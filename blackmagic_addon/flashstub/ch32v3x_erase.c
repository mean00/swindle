#include "ch32v3x.h"

/*
 */
bool ch32v3x_erase(uint32_t addr, size_t len)
{
    //(void);
    (void)addr;
    (void)len;

    uint32_t cur_addr = addr;
    uint32_t end_addr = cur_addr + len;
    while (cur_addr < end_addr)
    {
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_CH32_FASTERASE);
        WRITE__REG(->t, ADDR, cur_addr);
        ch32v3x_ctl_set(CH32V3XX_FMC_CTL_START);
        ch32v3x_wait_not_busy();
        ch32v3x_ctl_clear(CH32V3XX_FMC_CTL_CH32_FASTERASE);
        cur_addr += 256;
    }
    EXIT_OK();
    return true;
}

//
