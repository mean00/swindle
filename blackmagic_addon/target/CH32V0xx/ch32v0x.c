/*
 * This file is part of the Black Magic Debug project.
 * WCH CH32V0xx target driver.
 *
 * This file is derived from upstream blackmagic/src/target/ch32vx.c, trimmed to
 * the CH32V0xx (single-wire SDI) part only: upstream ch32vx.c also carries the
 * CH32V2xx/CH32V3xx families, which this project already handles in its own
 * addon driver (blackmagic_addon/target/CH32V3xx/ch32v3xx.c). Keeping only the
 * CH32V0xx content here avoids that overlap entirely: the V2xx/V3xx logic stays
 * out of this build, and blackmagic/src/target/ch32vx.c is never compiled.
 *
 * A RISC-V hart is attached by riscv_dmi_init() over the WCH SDI transport
 * (native: bmp_sdiTap_rp2040.cpp sdi_scan(); hosted: the same sequence driven
 * from Rust over the class-I RPC leg). riscv32_probe() dispatches on the WCH
 * designer code to ch32v003x_probe(), implemented below.
 *
 * Erase and programming are done by driving the on-chip FPEC registers through
 * the DM system bus (chapter 16 of the CH32V003 reference manual), so no flash
 * stub has to be uploaded into and run from target SRAM.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/* This file implements RISC-V CH32V0xx (single-wire SDI) target functions */

#include "general.h"
#include "target.h"
#include "target_internal.h"
#include "buffer_utils.h"
#include "riscv_debug.h"
#include "gdb_packet.h"

/*
 * IDCODE register (read through the DM system-bus access at 0x1ffff7c4)
 * [31:16] - REVID
 * [15:0]  - DEVID
 */
#define CH32V003X_IDCODE 0x1ffff7c4U
#define CH32V0X_IDCODE_MASK 0x0ffffff0fU

/* Electronic Signature (ESIG) registers */
#define CH32V0X_ESIG_FLASH_CAP 0x1ffff7e0U /* Flash capacity register, 16 bits, KiB units */
#define CH32V0X_ESIG_UID1 0x1ffff7e8U      /* Unique ID register, bits 0:31 */
#define CH32V0X_ESIG_UID2 0x1ffff7ecU      /* Unique ID register, bits 32:63 */
#define CH32V0X_ESIG_UID3 0x1ffff7f0U      /* Unique ID register, bits 64:95 */

#define RAM_SIZE 2
#define RAM_ADDRESS 0x20000000UL
/*
 * The 0x0000 0000 region is a boot alias ("Aliased to Flash or system memory",
 * chapter 1.2/option byte MODE) and is what a CH32V003 ELF is linked at, so it
 * is the window exposed to GDB. The FPEC itself only decodes the physical
 * CodeFlash window 0x0800 0000-0x0800 3FFF (table 16-1), so every address
 * handed to FLASH_ADDR or written for the fast-program buffer load has to be
 * translated by FLASH_OFFSET first.
 */
#define FLASH_ADDRESS 0x00000000UL
#define FLASH_OFFSET 0x08000000UL

/*
 * Flash programming (CH32V003 Reference Manual V1.3, chapter 16 "FLASH").
 *
 * The FPEC (flash program/erase controller) registers are memory mapped and are
 * driven from the debugger through the debug module system bus, the very same
 * access path used for the ESIG/IDCODE reads above. Two lock layers gate
 * programming: FPEC/FLASH_CTLR (unlocked via FLASH_KEYR, 16.4.2) and the fast
 * programming mode (unlocked via FLASH_MODEKEYR, 16.4.5). Erase and programming
 * are done in "fast page" mode (16.4.6/16.4.7), which works in 64 byte pages --
 * the same granularity exposed to the GDB flash layer as the flash block size.
 */

/* Table 16-2: FPEC register file */
#define CH32V0X_FLASH_CONTROLLER_ADDRESS 0x40022000U

/* 16.3.2/16.4.2: FPEC unlock keys, written in order to FLASH_KEYR */
#define CH32V0X_FLASH_KEY1 0x45670123U
#define CH32V0X_FLASH_KEY2 0xcdef89abU

/* 16.3.4: FLASH_STATR flags, EOP and WRPRTERR are cleared by writing 1 to them */
#define CH32V0X_FMC_STAT_BUSY (1U << 0U)     /* Flash operation in progress */
#define CH32V0X_FMC_STAT_WRPRTERR (1U << 4U) /* Write protection error */
#define CH32V0X_FMC_STAT_EOP (1U << 5U)      /* End of operation */

/* 16.3.5: FLASH_CTLR control bits */
#define CH32V0X_FMC_CTL_PG (1U << 0U)       /* Standard (half word) programming */
#define CH32V0X_FMC_CTL_PER (1U << 1U)      /* Standard page (1 KiB) erase */
#define CH32V0X_FMC_CTL_MER (1U << 2U)      /* Whole chip erase */
#define CH32V0X_FMC_CTL_STRT (1U << 6U)     /* Start erase/program, cleared by hardware */
#define CH32V0X_FMC_CTL_LOCK (1U << 7U)     /* FPEC and FLASH_CTLR locked */
#define CH32V0X_FMC_CTL_FLOCK (1U << 15U)   /* Fast programming/erase mode locked */
#define CH32V0X_FMC_CTL_FTPG (1U << 16U)    /* Fast page programming (64 B) */
#define CH32V0X_FMC_CTL_FTER (1U << 17U)    /* Fast page erase (64 B), "PAGE_ER" in 16.4.7 */
#define CH32V0X_FMC_CTL_BUFLOAD (1U << 18U) /* Cache the written word into the 64 B buffer */
#define CH32V0X_FMC_CTL_BUFRST (1U << 19U)  /* Reset (clear) the 64 B buffer */

/* 16.4.6/16.4.7: fast page granularity and the 32 bit buffer load width */
#define CH32V0X_FLASH_PAGE_BYTES 64U
#define CH32V0X_FLASH_WORD_BYTES 4U
#define CH32V0X_FLASH_PAGE_WORDS (CH32V0X_FLASH_PAGE_BYTES / CH32V0X_FLASH_WORD_BYTES)

/* FPEC operation timeout, generous compared to the ~1 ms page program/erase times */
#define CH32V0X_FLASH_TIMEOUT_MS 50U

/* Table 16-2 register file layout, byte offsets in the comments */
typedef struct
{
    uint32_t ACTLR;         /* 0x00 control */
    uint32_t KEYR;          /* 0x04 FPEC key */
    uint32_t OBKEYR;        /* 0x08 option byte key */
    uint32_t STATR;         /* 0x0c status */
    uint32_t CTLR;          /* 0x10 configuration */
    uint32_t ADDR;          /* 0x14 address */
    uint32_t RESERVED_18;   /* 0x18 */
    uint32_t OBR;           /* 0x1c option byte */
    uint32_t WPR;           /* 0x20 write protection */
    uint32_t MODEKEYR;      /* 0x24 extended (fast mode) key */
    uint32_t BOOT_MODEKEYR; /* 0x28 BOOT area key */
} ch32v0x_flash_s;

#define READ_FLASH_REG(target, reg)                                                                                    \
    target_mem32_read32(target, CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg))
#define WRITE_FLASH_REG(target, reg, value)                                                                            \
    target_mem32_write32(target, CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg), value)

static const command_s ch32v0x_cmd_list[] = {
    {NULL, NULL, NULL},
};

static size_t ch32v0x_read_flash_size(target_s *const target)
{
    return target_mem32_read16(target, CH32V0X_ESIG_FLASH_CAP);
}
static const size_t ch32v0_erase_sizes[] = {CH32V0X_FLASH_PAGE_BYTES, 0};

static void ch32v0x_read_uid(target_s *const target, uint8_t *const uid)
{
    for (size_t uid_reg_offset = 0; uid_reg_offset < 12U; uid_reg_offset += 4U)
        write_be4(uid, uid_reg_offset, target_mem32_read32(target, CH32V0X_ESIG_UID1 + uid_reg_offset));
}
/*
 * 16.3.5: read-modify-write of FLASH_CTLR, the FPEC has no atomic set/clear.
 * LOCK and FLOCK only latch a written 1, so ORing never releases them.
 */
static void ch32v0x_flash_ctl_set(target_s *const target, const uint32_t bits)
{
    WRITE_FLASH_REG(target, CTLR, READ_FLASH_REG(target, CTLR) | bits);
}

/* 16.4.2/16.4.5: open the FPEC lock and, if needed, the fast programming lock */
static bool ch32v0x_flash_unlock(target_s *const target)
{
    if (READ_FLASH_REG(target, CTLR) & CH32V0X_FMC_CTL_LOCK)
    {
        WRITE_FLASH_REG(target, KEYR, CH32V0X_FLASH_KEY1);
        WRITE_FLASH_REG(target, KEYR, CH32V0X_FLASH_KEY2);
    }

    /* The fast mode lock is separate and may still be set once LOCK is open */
    if (READ_FLASH_REG(target, CTLR) & CH32V0X_FMC_CTL_FLOCK)
    {
        WRITE_FLASH_REG(target, MODEKEYR, CH32V0X_FLASH_KEY1);
        WRITE_FLASH_REG(target, MODEKEYR, CH32V0X_FLASH_KEY2);
    }

    const uint32_t ctlr = READ_FLASH_REG(target, CTLR);
    if (ctlr & (CH32V0X_FMC_CTL_LOCK | CH32V0X_FMC_CTL_FLOCK))
    {
        DEBUG_ERROR("%s: CH32V0x FPEC unlock failed (FLASH_CTLR = 0x%08" PRIx32 ")\n", __func__, ctlr);
        return false;
    }
    return true;
}

/* 16.4.3/16.4.4: an operation is over once BSY is cleared */
static bool ch32v0x_flash_wait_not_busy(target_s *const target)
{
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, CH32V0X_FLASH_TIMEOUT_MS);

    uint32_t status = READ_FLASH_REG(target, STATR);
    while ((status & CH32V0X_FMC_STAT_BUSY) && !platform_timeout_is_expired(&timeout))
        status = READ_FLASH_REG(target, STATR);

    if (status & CH32V0X_FMC_STAT_BUSY)
    {
        DEBUG_ERROR("%s: CH32V0x FPEC busy timeout (FLASH_STATR = 0x%08" PRIx32 ")\n", __func__, status);
        return false;
    }
    return true;
}

/* 16.3.4: consume EOP (write 1 to clear) and report a write protection error */
static bool ch32v0x_flash_check_complete(target_s *const target, const target_addr_t addr)
{
    const uint32_t status = READ_FLASH_REG(target, STATR);

    /* Writing FLASH_STATR is only allowed with no operation in progress (BSY = 0) */
    WRITE_FLASH_REG(target, STATR, CH32V0X_FMC_STAT_EOP | CH32V0X_FMC_STAT_WRPRTERR);

    if (status & CH32V0X_FMC_STAT_WRPRTERR)
    {
        DEBUG_ERROR("%s: CH32V0x flash write protection error at 0x%08" PRIx32 "\n", __func__, (uint32_t)addr);
        return false;
    }
    return true;
}

/* 16.4.7: fast page erase, one 64 byte page per operation */
static bool ch32v0x_flash_erase(target_flash_s *const flash, target_addr_t addr, const size_t len)
{
    target_s *const target = flash->t;

    if (!ch32v0x_flash_unlock(target) || !ch32v0x_flash_wait_not_busy(target))
        return false;

    const uint32_t ctlr_saved = READ_FLASH_REG(target, CTLR);

    /* Step 4: enter fast page erase mode */
    ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTER);

    bool result = true;
    for (size_t offset = 0U; offset < len; offset += CH32V0X_FLASH_PAGE_BYTES)
    {
        const target_addr_t page_addr = (addr + offset) | FLASH_OFFSET;

        /* Steps 5-6: address the page and start the erase */
        WRITE_FLASH_REG(target, ADDR, (uint32_t)page_addr);
        ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_STRT);

        /* Step 7: wait for BSY to clear, then consume EOP and check for errors */
        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, page_addr))
        {
            result = false;
            break;
        }
    }

    /* Step 8: leave fast page erase mode, also restoring what we found */
    WRITE_FLASH_REG(target, CTLR, ctlr_saved);
    return result;
}

/* 16.4.6: fast page programming, one 64 byte page per operation */
static bool ch32v0x_flash_write(target_flash_s *const flash, target_addr_t dest, const void *const src,
                                const size_t len)
{
    target_s *const target = flash->t;
    const uint8_t *const data = src;

    /* A fast page is loaded into the internal 64 byte buffer and programmed whole */
    if ((dest % CH32V0X_FLASH_PAGE_BYTES) || (len % CH32V0X_FLASH_PAGE_BYTES))
    {
        DEBUG_ERROR("%s: CH32V0x fast programming needs 64 byte aligned pages (0x%08" PRIx32 " + %zu)\n", __func__,
                    (uint32_t)dest, len);
        return false;
    }

    if (!ch32v0x_flash_unlock(target) || !ch32v0x_flash_wait_not_busy(target))
        return false;

    const uint32_t ctlr_saved = READ_FLASH_REG(target, CTLR);

    /* Step 4: enter fast page programming mode */
    ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTPG);

    bool result = true;
    for (size_t offset = 0U; result && offset < len; offset += CH32V0X_FLASH_PAGE_BYTES)
    {
        const target_addr_t page_addr = (dest + offset) | FLASH_OFFSET;

        /* Steps 5-6: clear the internal 64 byte buffer */
        ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_BUFRST);
        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, page_addr))
        {
            result = false;
            break;
        }

        /* Steps 7-9: load the page four bytes at a time through BUFLOAD */
        for (size_t word = 0U; word < CH32V0X_FLASH_PAGE_WORDS; word++)
        {
            const size_t word_offset = word * CH32V0X_FLASH_WORD_BYTES;
            const target_addr_t word_addr = page_addr + word_offset;
            /* The GDB flash layer has already padded the page tail with flash->erased */
            const uint32_t value = read_le4(data, offset + word_offset);

            if (target_mem32_write32(target, (uint32_t)word_addr, value))
            {
                result = false;
                break;
            }

            ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_BUFLOAD);
            if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, word_addr))
            {
                result = false;
                break;
            }
        }
        if (!result)
            break;

        /* Steps 10-12: program the buffered page into flash */
        WRITE_FLASH_REG(target, ADDR, (uint32_t)page_addr);
        ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_STRT);

        if (!ch32v0x_flash_wait_not_busy(target) || !ch32v0x_flash_check_complete(target, page_addr))
            result = false;
    }

    /* Step 13: leave fast page programming mode, also restoring what we found */
    WRITE_FLASH_REG(target, CTLR, ctlr_saved);
    return result;
}

static void ch32v0x_add_flash(target_s *target, const uint32_t addr, const size_t length, const size_t erasesize,
                              const size_t writesize)
{
    target_flash_s *flash = calloc(1, sizeof(*flash));
    if (!flash)
    { /* calloc failed: heap exhaustion */
        DEBUG_ERROR("calloc: failed in %s\n", __func__);
        return;
    }
    flash->start = addr;
    flash->length = length;
    flash->blocksize = erasesize;
    flash->erasesizes = ch32v0_erase_sizes;
    flash->writesize = writesize;
    /* Buffered partial pages are padded with the erased state by the flash layer */
    flash->erased = 0xff;
    flash->erase = ch32v0x_flash_erase;
    flash->write = ch32v0x_flash_write;
    target_add_flash(target, flash);
}
//
static bool small_ch32v30_write_page(target_s *target, uint32_t addr, const uint8_t *src, uint32_t page_size)
{
    if (page_size != CH32V0X_FLASH_PAGE_BYTES)
    {
        DEBUG_ERROR("CH32V0: Wrong page size \n");
        return false;
    }
    if ((addr & (CH32V0X_FLASH_PAGE_BYTES - 1)) != 0)
    {
        DEBUG_ERROR("CH32V0: Wrong page alignment \n");
        return false;
    }
    return ch32v0x_flash_write(target->flash, addr, src, page_size);
}
static bool small_ch32v30_erase_page(target_s *target, uint32_t addr)
{
    if ((addr & (CH32V0X_FLASH_PAGE_BYTES - 1)) != 0)
    {
        DEBUG_ERROR("CH32V0: Erase: Wrong page alignment \n");
        return false;
    }
    return ch32v0x_flash_erase(target->flash, addr, 64);
}
static uint32_t small_ch32v30_page_size(target_s *t)
{
    return CH32V0X_FLASH_PAGE_BYTES;
}

static const sw_breakpoint_helpers ch32v0_sw_breakpoint_helper = {.page_size = small_ch32v30_page_size,
                                                                  .page_erase = small_ch32v30_erase_page,
                                                                  .page_write = small_ch32v30_write_page};

/* RISC-V Debug Module Registers and Bits */
#define RV_DM_CONTROL 0x10U
#define RV_DM_CTRL_HALT_REQ        (1U << 31U)
#define RV_DM_CTRL_HART_ACK_RESET  (1U << 28U)
#define RV_DM_CTRL_SYSTEM_RESET    (1U << 1U)
#define RV_DM_STAT_ALL_RESET      (1U << 19U)

static void ch32v003_reset(target_s *const target)
{
    riscv_hart_s *const hart = riscv_hart_struct(target);

    /* 1. Assert ndmreset (System Reset via DM) */
    riscv_dm_write(hart->dbg_module, RV_DM_CONTROL, hart->hartsel | RV_DM_CTRL_SYSTEM_RESET);
    
    /* 2. Wait for the core to acknowledge the reset state */
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 500U);
    do {
        uint32_t status = 0;
        if (riscv_dm_read(hart->dbg_module, 0x11U /* RV_DM_STATUS */, &status) &&
            (status & RV_DM_STAT_ALL_RESET))
            break;
    } while (!platform_timeout_is_expired(&timeout));

    /* 3. Release ndmreset AND assert haltreq simultaneously 
     * This instructs the DM to catch the CPU at the reset vector before it 
     * executes any instructions (e.g., persistent ebreaks in flash).
     */
    riscv_dm_write(hart->dbg_module, RV_DM_CONTROL, hart->hartsel | RV_DM_CTRL_HALT_REQ);

    /* 4. Acknowledge the reset */
    riscv_dm_write(hart->dbg_module, RV_DM_CONTROL, hart->hartsel | RV_DM_CTRL_HART_ACK_RESET | RV_DM_CTRL_HALT_REQ);

    /* 5. Cleanups carried over from standard riscv_reset() */
    if (hart->dbg_module->dmi_bus->invalidate_caches)
        hart->dbg_module->dmi_bus->invalidate_caches(hart->dbg_module->dmi_bus);
    
    target_check_error(target);
}

bool ch32v003x_probe(target_s *const target)
{
    const uint32_t idcode = target_mem32_read32(target, CH32V003X_IDCODE);

    switch (idcode & CH32V0X_IDCODE_MASK)
    {
    case 0x00300500U: /* CH32V003F4P6 */
    case 0x00310500U: /* CH32V003F4U6 */
    case 0x00320500U: /* CH32V003A4M6 */
    case 0x00330500U: /* CH32V003J4M6 */
        break;
    default:
        DEBUG_INFO("Unrecognized CH32V003x IDCODE: 0x%08" PRIx32 "\n", idcode);
        return false;
        break;
    }

    target->driver = "CH32V003";

    /* Override reset handler to fix halt-on-reset race condition */
    target->target_options |= TOPT_INHIBIT_NRST;
    target->reset = ch32v003_reset;

    const uint32_t flash_size = ch32v0x_read_flash_size(target);
    const uint32_t ram_size = RAM_SIZE;

    target->part_id = idcode;

    target_add_commands(target, ch32v0x_cmd_list, "CH32V0");
    target_mem_map_free(target);
    target_add_ram32(target, RAM_ADDRESS, ram_size * 1024U);
    ch32v0x_add_flash(target, FLASH_ADDRESS, (size_t)flash_size * 1024U, CH32V0X_FLASH_PAGE_BYTES,
                      CH32V0X_FLASH_PAGE_BYTES);
    target->sw_breakpoint_helpers = &ch32v0_sw_breakpoint_helper;
    /*
     * A CH32V003 has no usable hardware breakpoints, so say so to the GDB layer
     * (target_has_hw_breakpoint() is just !no_hw_breakpoint, and target_s comes
     * from calloc(), so this flag defaults to false). This is what makes the
     * Rust Z0 handler route GDB software breakpoints through the mass-write
     * helpers registered above (read page / erase page / reprogram page) instead
     * of taking the "has hardware breakpoint" branch, which patches the opcode
     * with a plain memory write. A plain write cannot program flash: the FPEC is
     * locked and PG is not set, so the write is silently ignored, no ebreak ever
     * lands in the flash and the breakpoint never fires.
     */
    target->no_hw_breakpoint = true;
    DEBUG_WARN("CH32V003 family %s\n", target->driver);
    DEBUG_WARN("CH32V003x flash size: %" PRIu32 " ram size: %" PRIu32 "\n", (uint32_t)flash_size, ram_size);
    gdb_outf("\tDetected %s chip with %d k flash, %d k ram\n", target->driver, flash_size, ram_size);

    return true;
}
// EOF
