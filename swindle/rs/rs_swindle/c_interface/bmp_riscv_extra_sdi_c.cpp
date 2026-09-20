/**
 * @file bmp_riscv_extra_sdi_c.cpp
 * @brief 'mon sdi_wire' knobs: the SDI side of the riscv_extra seam
 *        (swindle/include/bmp_riscv_extra.h).
 *
 * Moved out of bmp_interface_c.cpp so the whole SDI support is the deletable file
 * set that header lists: this file goes with it, and
 * bmp_riscv_extra_sdi_stubs_c.cpp takes its place when SWINDLE_WITH_SDI=OFF.
 */
#include "bmp_riscv_extra.h"

extern "C"
{
    /* Read by the bit-bang SDI transport (bmp_sdiTap_ln_bitbang_riscv_extra.cpp) on
     * every mode entry, ignored by the others. Zero means "the default the transport
     * defines". */
    uint32_t bmp_sdi_wire_tbit_ns = 0;
    uint32_t bmp_sdi_wire_low1_ns = 0;
    uint32_t bmp_sdi_wire_low0_ns = 0;
    uint32_t bmp_sdi_wire_sample_ns = 0;
    bool bmp_sdi_wire_no_trim = false;

    void bmp_set_sdi_wire_c(uint32_t tbit_ns, uint32_t low1_ns, uint32_t low0_ns, uint32_t sample_ns, bool no_trim)
    {
        bmp_sdi_wire_tbit_ns = tbit_ns;
        bmp_sdi_wire_low1_ns = low1_ns;
        bmp_sdi_wire_low0_ns = low0_ns;
        bmp_sdi_wire_sample_ns = sample_ns;
        bmp_sdi_wire_no_trim = no_trim;
    }
}
// EOF
