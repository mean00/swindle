/*
 * CH32V205 in-SRAM flash programming stub.
 *
 * The debugger uploads this image, unlocks FPEC (FLASH_KEYR) and the fast mode
 * (FLASH_MODEKEYR), hands it `(dest, src, len)` and reads the exit code. `dest` is the
 * page-aligned flash address, `src` the stub's data window in SRAM and `len` a whole
 * number of 256 byte pages.
 *
 * Sequence: WCH's CH32V205 EVT `FLASH_ROM_WRITE` (`ch32v205_flash.c`, V1.0.3), i.e. the
 * buffer-based controller of the CH32V00x family -- see `ch32v205_write.h` for why this
 * cannot be the chapter 32 sequence `ch32v3x_write.c` uses.
 *
 * Every command is a plain FLASH_CTLR write that restates PAGE_PG rather than a
 * read-modify-write: STRT is RW0 and writing a stale LOCK/FLOCK back as a 1 re-locks the
 * FPEC until the next system reset.
 */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "riscv32_flashstub.h"
#include "ch32v205_write.h"

#define FLASH_REG(reg) (*(volatile uint32_t *)(CH32V205_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v205_flash_s, reg)))

static inline __attribute__((always_inline)) void flash_cmd(const uint32_t bits)
{
    FLASH_REG(CTLR) = CH32V205_FMC_CTL_FTPG | bits;
    (void)FLASH_REG(CTLR); /* dummy read: let the posted APB write land before STATR is polled */
}

static inline __attribute__((always_inline)) void flash_wait_not_busy(void)
{
    while (FLASH_REG(STATR) & CH32V205_FMC_STAT_BUSY)
        ;
}

/* Consume the RW1C flags and report a write protection error. Writing STATR is only
 * allowed with no operation in progress. */
static inline __attribute__((always_inline)) bool flash_check_complete(void)
{
    const bool ok = (FLASH_REG(STATR) & CH32V205_FMC_STAT_WRPRTERR) == 0U;
    FLASH_REG(STATR) = CH32V205_FMC_STAT_CLEAR;
    return ok;
}

/* Cache one 32 bit word: store it through the flash window, then BUF_LOAD. The fence keeps
 * the data store ordered before the CTLR write, which the posted APB bridge could
 * otherwise overtake. */
static inline __attribute__((always_inline)) void flash_load_word(const uint32_t addr, const uint8_t *src)
{
    *(volatile uint32_t *)addr = *(const uint32_t *)src;
    __asm__ volatile("fence" ::: "memory");
    flash_cmd(CH32V205_FMC_CTL_BUFLOAD);
    flash_wait_not_busy();
}

bool ch32v205_write(uint32_t dest, uint32_t adr_source, uint32_t len)
{
    const uint8_t *src = (const uint8_t *)adr_source;
    uint32_t addr = dest;
    const uint32_t end = dest + len;
    bool ok = true;

    /* RM 27.4.2/27.4.4: the four key writes must be consecutive, with nothing else to the
     * FPEC in between - a wrong sequence locks the controller and makes every access to it
     * return bus errors until the next system reset. */
    FLASH_REG(KEYR) = CH32V205_FLASH_KEY1;
    FLASH_REG(KEYR) = CH32V205_FLASH_KEY2;
    FLASH_REG(MODEKEYR) = CH32V205_FLASH_KEY1;
    FLASH_REG(MODEKEYR) = CH32V205_FLASH_KEY2;
    flash_wait_not_busy();

    while (addr < end)
    {
        /* Clear OPTER/PAGE_ER first, then enter page program mode and clear the 256 byte
         * buffer. Three separate writes, as the EVT does: a leftover mode bit makes the
         * next command illegal. */
        FLASH_REG(CTLR) = 0;
        FLASH_REG(CTLR) = CH32V205_FMC_CTL_FTPG;
        flash_cmd(CH32V205_FMC_CTL_BUFRST);
        flash_wait_not_busy();
        if (!flash_check_complete())
        {
            ok = false;
            break;
        }

        for (uint32_t word = 0; word < CH32V205_FLASH_PAGE_BYTES / CH32V205_FLASH_WORD_BYTES; word++)
        {
            flash_load_word(addr + word * CH32V205_FLASH_WORD_BYTES, src);
            src += CH32V205_FLASH_WORD_BYTES;
        }

        /* ADDR must hold the page base just before STRT */
        FLASH_REG(ADDR) = addr;
        flash_cmd(CH32V205_FMC_CTL_STRT);
        flash_wait_not_busy();
        if (!flash_check_complete())
        {
            ok = false;
            break;
        }

        addr += CH32V205_FLASH_PAGE_BYTES;
    }

    /* Leave page mode on both paths; LOCK/FLOCK stay as the driver left them. */
    FLASH_REG(CTLR) = 0;
    riscv_stub_exit(ok ? 1 : 0);
    return ok;
}
