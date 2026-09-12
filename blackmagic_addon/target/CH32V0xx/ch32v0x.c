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

#define CH32V003_FLASHSTUB 1

typedef struct
{
    uint32_t family;
    uint32_t page_size;
} ch32v0x_priv_s;

#include "ch32v0x_reg.h"

#define READ_FLASH_REG(target, reg)                                                                                    \
    target_mem32_read32(target, CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg))
#define WRITE_FLASH_REG(target, reg, value)                                                                            \
    target_mem32_write32(target, CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg), value)

#define WRITE_MEM(a, b)                                                                                                \
    {                                                                                                                  \
    }

static const command_s ch32v0x_cmd_list[] = {
    {NULL, NULL, NULL},
};

static size_t ch32v0x_read_flash_size(target_s *const target)
{
    return target_mem32_read16(target, CH32V0X_ESIG_FLASH_CAP);
}
static const size_t ch32v0_erase_sizes[] = {512, 0};

static void ch32v0x_read_uid(target_s *const target, uint8_t *const uid)
{
    for (size_t uid_reg_offset = 0; uid_reg_offset < 12U; uid_reg_offset += 4U)
        write_be4(uid, uid_reg_offset, target_mem32_read32(target, CH32V0X_ESIG_UID1 + uid_reg_offset));
}
/*
 * 16.3.5: write of FLASH_CTLR, the FPEC has no atomic set/clear.
 * LOCK and FLOCK only latch a written 1, so writing 0 doesn't release them.
 */
static void ch32v0x_flash_ctl_set(target_s *const target, const uint32_t bits)
{
    WRITE_FLASH_REG(target, CTLR, bits);
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
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)target->target_storage;
    uint32_t page_size = priv->page_size;

    if (!ch32v0x_flash_unlock(target) || !ch32v0x_flash_wait_not_busy(target))
        return false;

    const uint32_t ctlr_saved = READ_FLASH_REG(target, CTLR);

    /* Step 4: enter fast page erase mode */
    ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTER);

    bool result = true;
    for (size_t offset = 0U; offset < len; offset += page_size)
    {
        const target_addr_t page_addr = (addr + offset) | FLASH_OFFSET;

        /* Steps 5-6: address the page and start the erase */
        WRITE_FLASH_REG(target, ADDR, (uint32_t)page_addr);
        ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTER | CH32V0X_FMC_CTL_STRT);

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
static uint32_t ch_read_le4(const uint8_t *t)
{
    return (uint32_t)t[0] + ((uint32_t)t[1] << 8UL) + ((uint32_t)t[2] << 16UL) + ((uint32_t)t[3] << 24UL);
}

#include "ch32v0x_write.h"

/* 16.4.6: fast page programming, one 64 byte page per operation */
static bool ch32v0x_flash_write(target_flash_s *const flash, target_addr_t dest, const void *const src,
                                const size_t len)
{
    target_s *const target = flash->t;
    uint8_t *data = (uint8_t *)src;
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)target->target_storage;
    uint32_t page_size = priv->page_size;

    /* A fast page is loaded into the internal 64 byte buffer and programmed whole */
    if ((dest % page_size) || (len % page_size))
    {
        DEBUG_ERROR("%s: CH32V0x fast programming needs %" PRIu32 " byte aligned pages (0x%08" PRIx32 " + %zu)\n",
                    __func__, page_size, (uint32_t)dest, len);
        return false;
    }

    if (!ch32v0x_flash_unlock(target) || !ch32v0x_flash_wait_not_busy(target))
        return false;

    const uint32_t ctlr_saved = READ_FLASH_REG(target, CTLR);

    /* Step 4: enter fast page programming mode */
    ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTPG);

    bool result = ch32v0x_write_inner(target, (uint32_t)dest | FLASH_OFFSET, data, len, page_size);

    /* Step 13: leave fast page programming mode, also restoring what we found */
    WRITE_FLASH_REG(target, CTLR, ctlr_saved);
    return result;
}

#if CH32V003_FLASHSTUB
#include "flashstub/ch32v0x_write.stub"

#define STUB_CODE_LOCATION 0x20000000
#define STUB_DATA_LOCATION 0x20000200
#define STUB_STAK_LOCATION 0x20000780

static bool ch32v0x_flash_prepare_flashstub(target_flash_s *flash)
{
    target_mem32_write(flash->t, STUB_CODE_LOCATION, ch32v0x_write_bin, ch32v0x_write_bin_len);
    return true;
}

static bool ch32v0x_flash_write_flashstub(target_flash_s *const flash, target_addr_t dest, const void *const src,
                                          const size_t len)
{
    // If the payload is small (e.g., GDB inserting a 2-byte ebreak),
    // use the native bit-banging function so we don't corrupt the user's live SRAM.
    if (len < 256)
    {
        return ch32v0x_flash_write(flash, dest, src, len);
    }

    target_s *const target = flash->t;
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)target->target_storage;
    uint32_t page_size = priv->page_size;

    if (!ch32v0x_flash_unlock(target) || !ch32v0x_flash_wait_not_busy(target))
        return false;
    const uint32_t ctlr_saved = READ_FLASH_REG(target, CTLR);

    uint32_t addr = (uint32_t)dest | FLASH_OFFSET;
    size_t remaining = len;
    const uint8_t *src_ptr = (const uint8_t *)src;

    while (remaining > 0)
    {
        uint32_t chunk = remaining > 1024 ? 1024 : remaining;

        target_mem32_write(target, STUB_DATA_LOCATION, src_ptr, chunk);

        ch32v0x_flash_ctl_set(target, CH32V0X_FMC_CTL_FTPG);

        // Set up a valid stack pointer at the top of the 2KB SRAM before running the stub
        uint32_t sp = STUB_STAK_LOCATION;
        target->reg_write(target, RISCV_REG_SP, &sp, 4);
        bool stub_success = riscv32_run_stub(flash->t, STUB_CODE_LOCATION, addr, STUB_DATA_LOCATION, chunk, page_size);
        if (!stub_success)
        {
            DEBUG_ERROR("CH32 Write Error at 0x%x\n", addr);
        }

        target->halt_request(target);

        if (!stub_success)
        {
            DEBUG_ERROR("CH32V003 Flash stub failed at 0x%08" PRIx32 " (MARK = %" PRIu32 ")\n", addr, 0);
            WRITE_FLASH_REG(target, CTLR, ctlr_saved);
            return false;
        }
        addr += chunk;
        src_ptr += chunk;
        remaining -= chunk;
    }

    WRITE_FLASH_REG(target, CTLR, ctlr_saved);
    return true;
}
#endif

static void ch32v0x_add_flash(target_s *target, const uint32_t addr, const size_t length, const size_t erasesize,
                              const size_t writesize, uint32_t family)
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
#if CH32V003_FLASHSTUB
    if (family == 3)
    {
        flash->prepare = ch32v0x_flash_prepare_flashstub;
        flash->write = ch32v0x_flash_write_flashstub;
    }
    else
    {
        flash->write = ch32v0x_flash_write;
    }
#else
    flash->write = ch32v0x_flash_write;
#endif
    target_add_flash(target, flash);
}
//
static bool small_ch32v30_write_page(target_s *target, uint32_t addr, const uint8_t *src, uint32_t page_size)
{
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)target->target_storage;
    if (page_size != priv->page_size)
    {
        DEBUG_ERROR("CH32V0: Wrong page size \n");
        return false;
    }
    if ((addr & (priv->page_size - 1)) != 0)
    {
        DEBUG_ERROR("CH32V0: Wrong page alignment \n");
        return false;
    }
    return ch32v0x_flash_write(target->flash, addr, src, page_size);
}
static bool small_ch32v30_erase_page(target_s *target, uint32_t addr)
{
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)target->target_storage;
    if ((addr & (priv->page_size - 1)) != 0)
    {
        DEBUG_ERROR("CH32V0: Erase: Wrong page alignment \n");
        return false;
    }
    return ch32v0x_flash_erase(target->flash, addr, priv->page_size);
}
static uint32_t small_ch32v30_page_size(target_s *t)
{
    ch32v0x_priv_s *priv = (ch32v0x_priv_s *)t->target_storage;
    return priv->page_size;
}

static const sw_breakpoint_helpers ch32v0_sw_breakpoint_helper = {.page_size = small_ch32v30_page_size,
                                                                  .page_erase = small_ch32v30_erase_page,
                                                                  .page_write = small_ch32v30_write_page};

/* RISC-V Debug Module Registers and Bits */
#define RV_DM_CONTROL 0x10U
#define RV_DM_CTRL_HALT_REQ (1U << 31U)
#define RV_DM_CTRL_HART_ACK_RESET (1U << 28U)
#define RV_DM_CTRL_SYSTEM_RESET (1U << 1U)
#define RV_DM_STAT_ALL_RESET (1U << 19U)

static void ch32v003_reset(target_s *const target)
{
    riscv_hart_s *const hart = riscv_hart_struct(target);

    /* 1. Assert ndmreset (System Reset via DM) */
    riscv_dm_write(hart->dbg_module, RV_DM_CONTROL, hart->hartsel | RV_DM_CTRL_SYSTEM_RESET);

    /* 2. Wait for the core to acknowledge the reset state */
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 500U);
    do
    {
        uint32_t status = 0;
        if (riscv_dm_read(hart->dbg_module, 0x11U /* RV_DM_STATUS */, &status) && (status & RV_DM_STAT_ALL_RESET))
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
    uint32_t ram_size = RAM_SIZE;
    uint32_t page_size = 64;
    uint32_t family = 0;

    switch (idcode & CH32V0X_IDCODE_MASK)
    {
    case CH32V0X_IDCODE_MASK: /* CH32V006xxxx */
        // In case that OTP was not set full of 1 , if the flash size is 62 kB
        // we assume it is a ch32v006
        if (ch32v0x_read_flash_size(target) == 62)
        {
            target->driver = "CH32V006";
            ram_size = 8; // all have 8kB ??
            page_size = 256;
            family = 6;
        }
        else
            return false;
        break;
    case 0x00300500U:        /* CH32V003F4P6 */
    case 0x00310500U:        /* CH32V003F4U6 */
    case 0x00320500U:        /* CH32V003A4M6 */
    case 0x00330500U:        /* CH32V003J4M6 */
        ram_size = RAM_SIZE; // all have 2kB
        target->driver = "CH32V003";
        family = 3;
        page_size = 64;
        break;
    default:
        DEBUG_INFO("Unrecognized CH32V003x IDCODE: 0x%08" PRIx32 "\n", idcode);
        return false;
        break;
    }

    ch32v0x_priv_s *priv = calloc(1, sizeof(*priv));
    priv->page_size = page_size;
    priv->family = family;
    target->target_storage = priv;

    /* Override reset handler to fix halt-on-reset race condition */
    target->target_options |= TOPT_INHIBIT_NRST;
    target->reset = ch32v003_reset;

    const uint32_t flash_size = ch32v0x_read_flash_size(target);

    target->part_id = idcode;

    target_add_commands(target, ch32v0x_cmd_list, "CH32V0");
    target_mem_map_free(target);
    target_add_ram32(target, RAM_ADDRESS, ram_size * 1024U);
    ch32v0x_add_flash(target, FLASH_ADDRESS, (size_t)flash_size * 1024U, 512, 512, family);
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
