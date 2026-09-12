#pragma once
#include <stdint.h>
#include "ch32v0x_reg.h"

#define MARK_ADDR 0x200001fc
#if 1
#define MARK(x)                                                                                                        \
    {                                                                                                                  \
    }
#else
#define MARK(x)                                                                                                        \
    {                                                                                                                  \
        WRITE_MEM(MARK_ADDR, x);                                                                                       \
    }
#endif
/*
 * 16.4.6 steps 4-14 / 18.4.5: the FPEC has no atomic set of a FLASH_CTLR bit, so
 * every command write restates FTPG instead of reading the register back and
 * or-ing the bit in. Nothing needs preserving that way: the only other writable
 * bits are mode bits which must be 0 (PG/PER/MER) and the RW1 bits
 * (STRT/LOCK/FLOCK) which must never be written back as a 1 -- a stale LOCK or
 * FLOCK re-locks the FPEC until the next system reset (16.4.2/16.4.5). WCH's own
 * SDK (FLASH->CTLR = CR_PAGE_PG | CR_BUF_RST), the ch32fun flashtest example and
 * minichlink (both its debug-module path and its in-RAM stub) all write these
 * constants, never a read-modify-write.
 */
#define WRITE_CMD(bits) WRITE_FLASH_REG(target, CTLR, CH32V0X_FMC_CTL_FTPG | (bits))
#define READ_CTRL() READ_FLASH_REG(target, CTLR)

// To share this code, both the host and the stub must define these helper functions
// and the WRITE_FLASH_REG macro before including this section.

static inline bool ch32v0x_write_inner(void *target, uint32_t start_addr, const uint8_t *data, size_t len,
                                       uint32_t page_size)
{
    bool result = true;
    uint32_t dest = start_addr;
    MARK(0);
    for (size_t offset = 0U; result && offset < len; offset += page_size)
    {
        /* Step 4: enter fast page programming mode. A write of its own, before
         * BUFRST, as in the SDK examples and minichlink ("THIS IS REQUIRED"). */
        WRITE_FLASH_REG(target, CTLR, CH32V0X_FMC_CTL_FTPG);

        /* Steps 5-6: clear the internal buffer, then wait for BSY to clear */
        MARK(11);
        WRITE_CMD(CH32V0X_FMC_CTL_BUFRST);
        (void)READ_CTRL(); /* dummy read: lets the posted APB write land before STATR is polled */

        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, start_addr + offset))
        {
            result = false;
            break;
        }
        MARK(1);
        /* Steps 7-9: load the page one 32 bit word at a time through BUFLOAD */
        for (size_t word = 0U; word < page_size / CH32V0X_FLASH_WORD_BYTES; word++)
        {
            const uint32_t value = ch_read_le4(data);
            if (target_mem32_write32(target, dest, value))
            {
                result = false;
                break;
            }
#ifdef WAIT_BETWEEN_BUFLOAD
            /* Only defined for the stub build: it runs from the target's RAM at
             * full core speed, where the FPEC needs the documented step 8/9 wait
             * before the next word. On the debug module path a debug transaction
             * is orders of magnitude slower than a buffer load, so no wait is
             * needed there. The fence keeps the data store ordered before the
             * CTLR write, which the APB bridge could otherwise overtake. */
            __asm__ volatile("fence" ::: "memory");
#endif
            WRITE_CMD(CH32V0X_FMC_CTL_BUFLOAD);
#ifdef WAIT_BETWEEN_BUFLOAD
            (void)READ_CTRL(); /* dummy read: lets the posted APB write land before STATR is polled */
            if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, dest))
            {
                result = false;
                break;
            }
#endif
            dest += 4;
            data += 4;
        }
        MARK(2);
        if (!result)
            break;

        MARK(3);
        /* Steps 10-12: program the buffered page into flash */
#ifdef WAIT_BETWEEN_BUFLOAD
        __asm__ volatile("fence" ::: "memory");
#endif

        // Some chips like CH32V006 require the ADDR register to be explicitly set just before STRT,
        // rather than at the beginning of the buffer loading phase, and it must point to the page base.
        WRITE_FLASH_REG(target, ADDR, start_addr + offset);

        WRITE_CMD(CH32V0X_FMC_CTL_STRT);
        (void)READ_CTRL(); /* dummy read: lets the posted APB write land before STATR is polled */
        MARK(7);

        /* Steps 12-13: wait for BSY to clear, which indicates the page
         * programming is done, then consume EOP and check for errors. This is
         * the wait that actually spins: a page program takes about 1 ms. */
        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, start_addr + offset))
        {
            result = false;
        }
        MARK(4);
    }
    MARK(5);
    /* Step 14: FTPG stays set while programming continues page after page; both
     * callers clear it together with the rest of FLASH_CTLR once the whole
     * write (or chunk, for the stub) is done. */
    return result;
}
// EOF
