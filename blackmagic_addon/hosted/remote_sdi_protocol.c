/**
 * @file remote_sdi_protocol.c
 * @brief Hosted (RPC-forwarded) WCH CH32V0xx single-wire (SDI) debug attach.
 *
 * Hosted counterpart of the native `sdi_scan()` in
 * swindle/src/platform/rp2040/bmp_sdiTap_rp2040.cpp, and sibling of the hosted
 * RVSWD glue in remote_rv_protocol.c (`bmda_rvswd_scan2()`). The scan, the
 * target-list handling and the `riscv_dmi_s` bookkeeping all live in C against
 * the *real* blackmagic headers (riscv_debug.h / target.h / jep106.h) — there
 * is deliberately no Rust-side layout mirror of `struct riscv_dmi` to keep in
 * sync with blackmagic.
 *
 * Only the DM-access leaf functions are forwarded to Rust
 * (`remote_sdi_reset_rs()` / `remote_sdi_dm_read_rs()` /
 * `remote_sdi_dm_write_rs()` in hosted/rpc_host/remote_rpc.rs), which encode
 * them over the class-I RPC link to the probe's PIO SDI primitives.
 */
#include "general.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_internal.h"

#include "jep106.h"

#define DMSTATUS 0x11U

extern bool remote_sdi_reset_rs(void);
extern bool remote_sdi_dm_read_rs(uint32_t address, uint32_t *value);
extern bool remote_sdi_dm_write_rs(uint32_t address, uint32_t value);

/* Bounded retry budget for DMI accesses, mirroring the native SDI tap. SDI
 * transfers do not return a per-access DMI status word, so the retry loop only
 * guards against the transport being momentarily unresponsive (e.g. right
 * after an unlock/reset). */
#define SDI_DMI_MAX_ATTEMPTS 4U

/**
 * @brief RPC-backed `riscv_dmi_s.read` — one DM register read over the
 * class-I leg.
 *
 * @param dmi
 * @param address
 * @param value
 * @return true on success
 * @return false on failure
 */
bool remote_ch32_sdi_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    for (uint32_t attempt = 0U; attempt < SDI_DMI_MAX_ATTEMPTS; ++attempt) {
        if (remote_sdi_dm_read_rs(address, value)) {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    dmi->fault = RV_DMI_FAILURE;
    return false;
}

/**
 * @brief RPC-backed `riscv_dmi_s.write` — one DM register write over the
 * class-I leg.
 *
 * @param dmi
 * @param address
 * @param value
 * @return true on success
 * @return false on failure
 */
bool remote_ch32_sdi_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    for (uint32_t attempt = 0U; attempt < SDI_DMI_MAX_ATTEMPTS; ++attempt) {
        if (remote_sdi_dm_write_rs(address, value)) {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    dmi->fault = RV_DMI_FAILURE;
    return false;
}

/**
 * @brief Hosted SDI scan + full RISC-V target attach.
 *
 * Mirror of the native `sdi_scan()` (sdiTap) and of `bmda_rvswd_scan2()`:
 * unlock the CH32V0xx over RPC, gate on DMSTATUS, clear stale targets, then
 * hand `riscv_dmi_init()` an SDI-backed `riscv_dmi_s` (designer = WCH) so the
 * C RISC-V framework discovers the hart and runs the per-family probe
 * (riscv32_probe -> ch32v003x_probe). An unresponsive target leaves the SDI
 * bus idle-high, which reads back as all-ones.
 *
 * Invoked from Rust (`mon sdi_scan` -> bmp::sdi_scan); see the link-pull note
 * in remote_rv_protocol.c for why this object is kept in the hosted link.
 * @return true when a target responded to the unlock (attach attempted).
 */
bool bmda_sdi_scan2(void)
{
    /* Enter SDI debug mode on the remote probe (PIO upload, NRST pulse,
     * unlock). The native tap ignores the reset result and gates on the
     * DMSTATUS read below; keep that behaviour. */
    (void)remote_sdi_reset_rs();

    uint32_t status = 0;
    if (!remote_sdi_dm_read_rs(DMSTATUS, &status)) {
        DEBUG_ERROR("SDI : DMSTATUS read failed\n");
        return false;
    }
    if (status == 0xFFFFFFFFUL) {
        DEBUG_ERROR("SDI : no target responding (DMSTATUS=0x%x)\n", (unsigned)status);
        return false;
    }
    DEBUG_ERROR("SDI : found target DMSTATUS=0x%x, attaching RISC-V debug module\n", (unsigned)status);

    target_list_free();

    riscv_dmi_s *const dmi = (riscv_dmi_s *)malloc(sizeof(*dmi));
    if (!dmi) {
        /* allocation failed: heap exhaustion */
        DEBUG_ERROR("SDI : dmi allocation failed in %s\n", __func__);
        return false;
    }
    memset(dmi, 0, sizeof(*dmi));
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    /* SDI frames carry a 7-bit DMI address field (start+7addr+RW header, see
     * make_sdi_header), matching wchlink_riscv_dtm.c. The tap never reads this
     * field (fixed-frame driver), but it must describe the bus correctly. */
    dmi->address_width = 7U;
    dmi->read = remote_ch32_sdi_dmi_read;
    dmi->write = remote_ch32_sdi_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}

// EOF
