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
#define WRITE_CTRL(x) WRITE_FLASH_REG(target, CTLR, base_ctlr | (x))
#define READ_CTRL() READ_FLASH_REG(target, CTLR)

// To share this code, both the host and the stub must define these helper functions
// and the WRITE_FLASH_REG macro before including this section.
static inline bool ch32v0x_write_inner(void *target, uint32_t start_addr, const uint8_t *data, size_t len,
                                       uint32_t ctlr_bufload)
{
    uint32_t base_ctlr = ctlr_bufload & ~CH32V0X_FMC_CTL_BUFLOAD;
    bool result = true;
    uint32_t zero = 0;
    uint32_t dest = start_addr;
    MARK(0);
    for (size_t offset = 0U; result && offset < len; offset += CH32V0X_FLASH_PAGE_BYTES)
    {
        /* Steps 5-6: clear the internal 64 byte buffer */
        MARK(11);
        WRITE_CTRL(CH32V0X_FMC_CTL_BUFRST);
        (void)READ_CTRL(); // Dummy read: flushes APB write buffer to ensure FMC asserts BSY before we poll STATR
        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, dest))
        {
            result = false;
            break;
        }
        MARK(1);
        /* Steps 7-9: load the page four bytes at a time through BUFLOAD */
        uint32_t src = 0;
        WRITE_FLASH_REG(target, ADDR, dest);
        for (size_t word = 0U; word < CH32V0X_FLASH_PAGE_WORDS; word++)
        {
            const uint32_t value = ch_read_le4(data);
            if (target_mem32_write32(target, dest, value))
            {
                result = false;
                break;
            }
            WRITE_CTRL(CH32V0X_FMC_CTL_BUFLOAD);
            (void)READ_CTRL(); // Dummy read: flushes APB write buffer to ensure FMC asserts BSY before we poll STATR
            dest += 4;
            data += 4;
#ifdef WAIT_BETWEEN_BUFLOAD
            if (!ch32v0x_flash_wait_not_busy(
                    target)) // this is only needed when running from ram, else we are faster than the flash ctrl
            {
                result = false;
                break;
            }
#endif
        }
        MARK(2);
        if (!result)
            break;

        MARK(3);
        /* Steps 10-12: program the buffered page into flash */
        WRITE_CTRL(CH32V0X_FMC_CTL_STRT);
        (void)READ_CTRL(); // Dummy read: flushes APB write buffer to ensure FMC asserts BSY before we poll STATR
        MARK(7);

        if (!ch32v0x_flash_wait_not_busy(target))
        {
            result = false;
        }
        else
        {
            MARK(8);
            if (!ch32v0x_flash_check_complete(target, dest))
            {
                result = false;
            }
        }
        MARK(4);
    }
    MARK(5);
    // end loop of 64 bytes block
    return result;
}
// EOF
