/**
 * @file bmp_riscv_extra_sdi_stubs_c.cpp
 * @brief Default (no SDI) definition of the 'mon sdi_wire' setter: the
 *        c_interface side of the riscv_extra seam (swindle/include/bmp_riscv_extra.h).
 *
 * Built instead of bmp_riscv_extra_sdi_c.cpp when SWINDLE_WITH_SDI=OFF, so that
 * the Rust 'mon sdi_wire' command still links. The knob variables themselves are
 * not defined here: with no SDI transport in the build nothing reads them.
 */
#include "bmp_riscv_extra.h"

extern "C" void bmp_set_sdi_wire_c(uint32_t tbit_ns, uint32_t low1_ns, uint32_t low0_ns, uint32_t sample_ns,
                                   bool no_trim)
{
    (void)tbit_ns;
    (void)low1_ns;
    (void)low0_ns;
    (void)sample_ns;
    (void)no_trim;
}
// EOF
