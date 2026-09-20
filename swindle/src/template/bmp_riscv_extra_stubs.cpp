/**
 * @file bmp_riscv_extra_stubs.cpp
 * @brief Default (no SDI) definitions of the WCH SDI seam declared in
 *        swindle/include/bmp_riscv_extra.h.
 *
 * Built instead of the SDI transport when SWINDLE_WITH_SDI=OFF (see the platform
 * CMakeLists.txt). It keeps every reference to the SDI entry points - the Rust
 * 'mon sdi_scan' / SDI RPC leaves and the SDI arm of the platform's
 * bmp_gpio_pinmode() - linked, with a body that fails or does nothing, so the
 * firmware is a plain SWD/RVSWD probe and the RISC-V stack is untouched. Same
 * role as swd_tap_stubs.cpp for the platforms whose SWD TAP is missing.
 */
#include "bmp_riscv_extra.h"

extern "C"
{
    /* No transport, so no probe: 'mon sdi_scan' reports the failure it already
     * knows how to report ("sdi_scan failed!"), and the native SDI RPC leaves
     * report a failed DM access. */
    bool sdi_scan()
    {
        return false;
    }
    bool bmp_sdi_dm_reset_c()
    {
        return false;
    }
    bool bmp_sdi_dm_read_c(const uint8_t adr, uint32_t *const value)
    {
        (void)adr;
        (void)value;
        return false;
    }
    bool bmp_sdi_dm_write_c(const uint8_t adr, const uint32_t value)
    {
        (void)adr;
        (void)value;
        return false;
    }
}

/* No SDI session to hand the pins to, so the pin mode dispatcher has nothing to
 * do: SWD and RVSWD configure their pins themselves. */
void sdi_pinmode_enter()
{
}
void sdi_pinmode_leave()
{
}
// EOF
