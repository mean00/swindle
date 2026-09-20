/**
 * @file bmp_riscv_extra.h
 * @brief Seam between the RISC-V debug stack and the optional WCH SDI feature.
 *
 * SDI (WCH CH32V0xx single-wire debug) is a leaf of the RISC-V support: the RVSWD
 * stack (bmp_rvTap_*.cpp / rvswd_template.h), the DM framework (riscv_debug.c /
 * riscv32.c) and every SWD target work without it. Everything the rest of the
 * firmware - the Rust commands, the platform taps, the pin mode dispatcher - may
 * reference from SDI is declared here, and is defined either by the SDI sources
 * (SWINDLE_WITH_SDI=ON) or by src/template/bmp_riscv_extra_stubs.cpp
 * (SWINDLE_WITH_SDI=OFF): removing the SDI files therefore leaves every build
 * target intact and only makes 'mon sdi_scan' report failure.
 *
 * The SDI sources are the files named *_riscv_extra.* below:
 *   swindle/src/template/sdi_template.h                        protocol, DM, DMI glue, sdi_scan
 *   swindle/src/platform/ln/bmp_sdiTap_ln_bitbang_riscv_extra.cpp
 *   swindle/src/platform/ln/bmp_sdiTap_ln_riscv_extra.cpp
 *   swindle/src/platform/rp2040/bmp_sdiTap_rp2040_riscv_extra.cpp
 *   swindle/src/platform/rp2040/bmp_tap_rp2040_riscv_extra.cpp PIO upload + pin hand-over
 *   swindle/rs/rs_swindle/c_interface/bmp_riscv_extra_sdi_c.cpp 'mon sdi_wire' knobs
 *   swindle/rs/rs_swindle/src/riscv_extra_sdi.rs               Rust half: the names below,
 *                                                             the mon commands, the Rust FFI
 *
 * The Rust half of the seam is selected the same way, by a cargo feature driven by
 * the same option (swindle/rs/CMakeLists.txt): with SWINDLE_WITH_SDI=ON rs_swindle
 * compiles riscv_extra_sdi.rs as its crate::riscv_extra module, with the option off
 * it compiles riscv_extra_sdi_stubs.rs instead. So the Rust names listed here
 * (sdi_scan, bmp_sdi_dm_reset_c, bmp_sdi_dm_read_c, bmp_sdi_dm_write_c,
 * bmp_set_sdi_wire_c) resolve either way, and the only Rust-side difference the
 * option makes is what 'mon sdi_scan' / 'mon sdi_wire' print.
 */
#pragma once

#include "stdint.h"

#ifdef __cplusplus
extern "C"
{
#endif

    /* Stage 1 of the WCH attach: the probe itself, defined once per SDI transport
     * by sdi_template.h. 'mon sdi_scan' and the native SDI RPC leaves call them. */
    bool sdi_scan();
    bool bmp_sdi_dm_reset_c();
    bool bmp_sdi_dm_read_c(const uint8_t adr, uint32_t *const value);
    bool bmp_sdi_dm_write_c(const uint8_t adr, const uint32_t value);

    /* The bit-bang waveform overrides ('mon sdi_wire'), in ns: the cell, the LOW
     * that carries a 1, the LOW that carries a 0 and the response sample point.
     * A zero leaves that number as the transport's own default, which is also the
     * power-on state. Only a transport that reads them needs them defined, so they
     * live with the SDI support (bmp_riscv_extra_sdi_c.cpp). */
    extern uint32_t bmp_sdi_wire_tbit_ns;
    extern uint32_t bmp_sdi_wire_low1_ns;
    extern uint32_t bmp_sdi_wire_low0_ns;
    extern uint32_t bmp_sdi_wire_sample_ns;
    extern bool bmp_sdi_wire_no_trim;

    /**
     * @brief Set the SDI wire timing ('mon sdi_wire'), in ns.
     *
     * A zero leaves that number as the SDI tap's own default, so a call only has to
     * name the numbers it wants to change. The tap notices the change and calibrates
     * again on its next mode entry, i.e. 'mon sdi_scan' is what applies a new set.
     *
     * This is the native leg. A hosted (Blackmagic-app) build does not define it:
     * the tap being tuned is the probe's, so the same numbers go over the class-I
     * SDI RPC as SDI.SET_WIRE (rpc_protocol.toml) to rpc_sdi_impl::set_wire, which
     * ends here on the probe. See riscv_extra_sdi.rs's two bmp_set_sdi_wire
     * variants and hosted/rpc_host/remote_rpc.rs::remote_sdi_set_wire_rs.
     */
    void bmp_set_sdi_wire_c(uint32_t tbit_ns, uint32_t low1_ns, uint32_t low0_ns, uint32_t sample_ns, bool no_trim);

#ifdef __cplusplus
}
#endif

/* Pin hand-over (C++ linkage, like bmp_gpio_pinmode() itself): the platform's
 * bmp_gpio_pinmode() calls these two when the SDI mode is selected and when it is
 * left. The default definitions do nothing, which is what an SWD/RVSWD-only build
 * wants - both taps set their own pins. */
void sdi_pinmode_enter();
void sdi_pinmode_leave();

// EOF
