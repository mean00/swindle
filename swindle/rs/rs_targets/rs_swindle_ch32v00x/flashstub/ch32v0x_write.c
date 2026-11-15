/*
 * CH32V00x in-SRAM flash programming stub.
 *
 * The debugger uploads this image, unlocks the FPEC, hands it
 * `_start(dest, src, len, page_size)` and reads back the block. It programs the
 * chunk with the fast page sequence of the CH32V003 Reference Manual V1.3,
 * 16.4.6, and publishes the common stub exit code.
 */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "riscv32_flashstub.h"
#include "ch32v0x_write.h"

/*
 * The stub runs on the target itself, so a controller register is just a
 * memory-mapped word and its address is the base plus the offset the register
 * file struct gives it (the same `offsetof` derivation the Rust driver makes
 * with `core::mem::offset_of!`).
 */
#define FLASH_REG(reg) (*(volatile uint32_t *)(CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg)))

/*
 * 16.4.6 steps 4-14: the FPEC has no atomic set of a FLASH_CTLR bit, so every
 * command write restates FTPG instead of reading the register back and or-ing
 * the bit in. Nothing needs preserving that way: the only other writable bits
 * are mode bits which must be 0 (PG/PER/MER) and the RW1 bits
 * (STRT/LOCK/FLOCK) which must never be written back as a 1 -- a stale LOCK or
 * FLOCK re-locks the FPEC until the next system reset (16.4.2/16.4.5). WCH's own
 * SDK (FLASH->CTLR = CR_PAGE_PG | CR_BUF_RST), the ch32fun flashtest example and
 * minichlink (both its debug-module path and its in-RAM stub) all write these
 * constants, never a read-modify-write.
 */
static inline __attribute__((always_inline)) void flash_cmd(const uint32_t bits)
{
    FLASH_REG(CTLR) = CH32V0X_FMC_CTL_FTPG | bits;
    (void)FLASH_REG(CTLR); /* dummy read: lets the posted APB write land before STATR is polled */
}

/* Steps 5-6/12-13: BSY clears when the FPEC has finished the current operation.
 * A page program takes about 1 ms, so this is the wait that actually spins. */
static inline __attribute__((always_inline)) void flash_wait_not_busy(void)
{
    while (FLASH_REG(STATR) & CH32V0X_FMC_STAT_BUSY)
        ;
}

/* Steps 12-13: consume EOP and report a write protection error. Writing STATR
 * is only allowed with no operation in progress (BSY = 0). */
static inline __attribute__((always_inline)) bool flash_check_complete(void)
{
    const bool ok = (FLASH_REG(STATR) & CH32V0X_FMC_STAT_WRPRTERR) == 0U;
    FLASH_REG(STATR) = CH32V0X_FMC_STAT_EOP | CH32V0X_FMC_STAT_WRPRTERR;
    return ok;
}

/* Steps 7-9: cache one 32 bit word of the page in the FPEC buffer. The stub runs
 * at full core speed, where the FPEC needs the documented step 8/9 wait before
 * the next word, so neither the fence nor the busy wait below is optional. The
 * fence keeps the data store ordered before the CTLR write, which the APB bridge
 * could otherwise overtake. */
static inline __attribute__((always_inline)) void flash_load_word(const uint32_t addr, const uint8_t *src)
{
    *(volatile uint32_t *)addr = *(const uint32_t *)src;
    __asm__ volatile("fence" ::: "memory");
    flash_cmd(CH32V0X_FMC_CTL_BUFLOAD);
    flash_wait_not_busy();
}

/* Program `len` bytes from `src` into flash at `page_base`, one `page_size` page
 * at a time. FTPG stays set while programming continues page after page; the
 * caller clears it together with the rest of FLASH_CTLR once the chunk is done. */
static bool flash_write_pages(uint32_t page_base, const uint8_t *src, size_t len, uint32_t page_size)
{
    for (size_t offset = 0; offset < len; offset += page_size)
    {
        const uint32_t page_addr = page_base + (uint32_t)offset;

        /* Step 4: enter fast page programming mode. A write of its own, before
         * BUFRST, as in the SDK examples and minichlink ("THIS IS REQUIRED"). */
        FLASH_REG(CTLR) = CH32V0X_FMC_CTL_FTPG;

        /* Steps 5-6: clear the internal buffer, then wait for BSY to clear */
        flash_cmd(CH32V0X_FMC_CTL_BUFRST);
        flash_wait_not_busy();
        if (!flash_check_complete())
            return false;

        /* Steps 7-9: load the page one 32 bit word at a time through BUFLOAD */
        for (uint32_t word = 0; word < page_size / CH32V0X_FLASH_WORD_BYTES; word++)
        {
            flash_load_word(page_addr + word * CH32V0X_FLASH_WORD_BYTES, src);
            src += CH32V0X_FLASH_WORD_BYTES;
        }

        /* Steps 10-12: program the buffered page into flash. Some chips like the
         * CH32V006 require the ADDR register to be explicitly set just before
         * STRT, rather than at the beginning of the buffer loading phase, and it
         * must point to the page base. */
        FLASH_REG(ADDR) = page_addr;
        flash_cmd(CH32V0X_FMC_CTL_STRT);
        flash_wait_not_busy();
        if (!flash_check_complete())
            return false;
    }
    return true;
}

/* Stub entry: a0 = dest, a1 = src, a2 = len, a3 = page_size (see
 * `RiscvHart::run_stub_with_stack_hold`). Interrupts are off before anything
 * else so nothing else runs from this core while its flash is being programmed. */
__attribute__((naked, noreturn)) void _start(uint32_t dest, uint32_t src, uint32_t len, uint32_t page_size)
{
    __asm__ volatile("csrci mstatus, 8\n");
    const bool ok = flash_write_pages(dest, (const uint8_t *)src, len, page_size);
    /* Publish the common stub exit code: 1 = the chunk is programmed. */
    riscv_stub_exit(ok);
}
