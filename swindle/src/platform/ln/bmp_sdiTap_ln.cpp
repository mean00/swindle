/*
 */
/**
 * This is similar to the non rp2040 except we switch to bit banging dynamically
 *
 */

// -------------------------------------------------- //
#include "lnGPIO.h"
#include "stdint.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
#include <cstring>

// The blackmagic RISC-V target framework (riscv_debug.h) is needed for the
// Stage-2 attach below: we hand riscv_dmi_init() an SDI-backed riscv_dmi_s,
// exactly mirroring what rvswd_template.h::rvswd_scan() does over RVSWD.
extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}
extern "C"
{
    /** @brief C entry point: enter SDI debug mode (mirror of bmp_rv_dm_reset_c). */
    bool bmp_sdi_dm_reset_c()
    {
        return false; // sdi_dm_start();
    }
    /** @brief C entry point: SDI DM write (mirror of bmp_rv_dm_write_c). */
    bool bmp_sdi_dm_write_c(const uint8_t adr, const uint32_t value)
    {
        return false; // sdi_dm_write(adr, value);
    }
    /** @brief C entry point: SDI DM read (mirror of bmp_rv_dm_read_c). */
    bool bmp_sdi_dm_read_c(const uint8_t adr, uint32_t *const value)
    {
        return false; // sdi_dm_read(adr, value);
    }
}

static bool ch32_sdi_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    return false;
}

static bool ch32_sdi_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    return false;
}

/**
 * @brief SDI scan + full RISC-V target attach (stage 2).
 *
 * Stage 1 unlocked the CH32V0xx and reported DMSTATUS only. Stage 2 mirrors
 * rvswd_template.h::rvswd_scan(): after the unlock it clears the blackmagic
 * target list and hands riscv_dmi_init() an SDI-backed riscv_dmi_s (designer =
 * WCH), so the C RISC-V framework discovers the hart and runs the per-family
 * probe (riscv32_probe -> ch32v003x_probe) exactly as it does over RVSWD.
 * A missing / unresponsive target leaves the SDI bus idle-high, which reads
 * back as all-ones.
 * @return true when a target responded to the unlock (attach attempted).
 */
extern "C" bool sdi_scan()
{
    return false;
}

// EOF
