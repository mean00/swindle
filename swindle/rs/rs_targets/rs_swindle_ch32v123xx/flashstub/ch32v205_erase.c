/*
 * CH32V205 in-SRAM flash erase stub.
 *
 * `(addr, len, mode)` with `mode` = PER (2 KiB sector), FTER (256 B page) or BER64 (64 KiB
 * block, address masked to the block base). Everything - the unlock sequence, the command,
 * the status polling - runs on the target core, because the CH32V205's debug module returns
 * unreliable values for multi-word reads and for the FPEC state registers, so the probe cannot
 * judge the outcome itself. Success/failure comes back as the stub exit code.
 */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "riscv32_flashstub.h"
#include "ch32v205_write.h"

#define FLASH_REG(reg) (*(volatile uint32_t *)(CH32V205_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v205_flash_s, reg)))

static inline __attribute__((always_inline)) void flash_wait_not_busy(void)
{
    while (FLASH_REG(STATR) & CH32V205_FMC_STAT_BUSY)
        ;
}

bool ch32v205_erase(uint32_t addr, uint32_t len, uint32_t mode)
{
    const uint32_t step = (mode == CH32V205_FMC_CTL_BER64) ? CH32V205_BLOCK64_BYTES
                        : (mode == CH32V205_FMC_CTL_PER)   ? CH32V205_SECTOR_BYTES
                                                           : CH32V205_FLASH_PAGE_BYTES;
    bool ok = true;

    /* RM 27.4.2/27.4.4: four consecutive key writes, nothing else to the FPEC in between. */
    FLASH_REG(KEYR) = CH32V205_FLASH_KEY1;
    FLASH_REG(KEYR) = CH32V205_FLASH_KEY2;
    FLASH_REG(MODEKEYR) = CH32V205_FLASH_KEY1;
    FLASH_REG(MODEKEYR) = CH32V205_FLASH_KEY2;
    flash_wait_not_busy();

    uint32_t cur = addr;
    const uint32_t end = addr + len;
    while (cur < end)
    {
        const uint32_t target =
            (mode == CH32V205_FMC_CTL_BER64) ? (cur & 0xFFFF0000U) : cur;

        /* The mode is selected with STRT still 0, then the address, then start (RM 27.4.3). */
        FLASH_REG(CTLR) = mode;
        FLASH_REG(ADDR) = target;
        FLASH_REG(CTLR) = mode | CH32V205_FMC_CTL_STRT;
        flash_wait_not_busy();

        const uint32_t stat = FLASH_REG(STATR);
        FLASH_REG(STATR) = CH32V205_FMC_STAT_CLEAR; /* EOP/WRPRTERR are RW1C */
        FLASH_REG(CTLR) = 0;
        if ((stat & CH32V205_FMC_STAT_WRPRTERR) != 0U)
        {
            ok = false;
            break;
        }
        cur += step;
    }

    riscv_stub_exit(ok ? 1 : 0);
    return ok;
}
