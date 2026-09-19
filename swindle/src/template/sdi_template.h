/**
 * @file sdi_template.h
 * @brief WCH SDI (single wire debug interface) protocol layer, included by the
 *        platform taps.
 *
 * Same idea as swd_template.h / rvswd_template.h: the protocol is written once
 * and the platform supplies the wire. What sits below is everything that is
 * independent of how the bit stream is generated - the DM/configuration
 * registers, the §2.4 shadow/commit protocol, the frame format, the CPBR
 * diagnostics, the DMI glue and the stage-2 RISC-V attach.
 *
 * The platform file must provide, before including this file:
 *
 *   sdiFrameRead(adr)        one whole DM read frame   -> uint32_t
 *   sdiFrameWrite(adr, val)  one whole DM write frame  -> void
 *   sdiWireHoldLow(ms)       §2.4(3) reset pulse: wire low for @p ms, released
 *
 * These three are text macros, exactly like DIR_INPUT()/SWD_WAIT_PERIOD() in
 * swd_template.h: the platform #defines them over its own transport, so the
 * frame-level code below expands to byte-identical source on every platform.
 * The seam is deliberately at frame granularity: a whole frame is ~37 us, so a
 * transport that is synchronous (LN: CPU cells / timer+DMA) and one that is
 * asynchronous (RP2040: PIO) cost the same here, and neither has to grow a
 * callout inside a bit cell.
 *
 * It also relies on what both platform taps already include: bmp_pinmode.h
 * (bmp_gpio_pinmode / BMP_PINMODE_SDI), pReset (SwdReset *), Logger(),
 * lnDelayMs()/lnDelayUs(), <cstring>, and the blackmagic RISC-V target
 * framework (jep106.h / riscv_debug.h / target_list_free) that sdi_scan() uses
 * for the stage-2 attach.
 */

/* WCH debug module (DM) register used by the transport (QingKeV2 debug manual,
 * ch. 3, Table 3-5): dmstatus [3:0] version (0010 = V0.13, reset value 0x2),
 * [7] authenticated. dmcontrol (0x10) is deliberately not written from here:
 * riscv_debug.c::riscv_dm_init() activates the DM itself with
 * RV_DM_CTRL_ACTIVE once the scan attaches. */
#define DMSTATUS 0x11

/* dmstatus version nibble / authenticated bit; the version is 2 for v0.13 (the
 * version this tap claims via RISCV_DEBUG_0_13). */
#define SDI_DMSTATUS_VERSION_0_13 2U
#define SDI_DMSTATUS_AUTHENTICATED (1U << 7)

/* SDI debug-interface registers (§2.3 Table 2-1). */
#define SDI_CPBR 0x7C     /* capability register: reports what actually took effect */
#define SDI_CFGR 0x7D     /* configuration register: carries the update mask */
#define SDI_SHDWCFGR 0x7E /* shadow configuration register: carries the value */

/* Configuration protocol (§2.4): a write only lands when the upper half word is
 * the key 0x5AA5. SHDWCFGR (0x7E) carries the *value* and CFGR (0x7D) the *field
 * mask* that commits it ("first set the corresponding bit of SHDWCFGR and then
 * set the corresponding bit field of CFGR to set the corresponding
 * configuration bit of the shadow configuration register to take effect, while
 * other configuration bits remain unchanged"). The bits set in CFGR are an
 * *update mask*, not a value: §2.4(1) writes SHDWCFGR = CFGR = 0x5AA50400, where
 * mask bit 10 commits OUTEN from a shadow value that also enables it. §2.4(2)
 * needs two *different* words for the same reason - SHDWCFGR = 0x5AA50000 (value)
 * and CFGR = 0x5AA50003 (mask [1:0] = 0b11) - and blindly applying that pair is
 * what cleared OUTEN and took the link down (see the caveat below).
 *
 * MEASURED CAVEAT (CH32V003): a SHDWCFGR write replaces the *whole* shadow
 * value, so a value with bit 10 clear switches the slave output driver off and
 * takes the entire SDI link down (DMSTATUS then reads the idle-high all-ones
 * pattern and the scan fails). Every SHDWCFGR write below therefore carries
 * SDI_CFGR_OUTEN. */
#define SDI_KEY 0x5AA5U
#define SDI_KEYED(data) (((uint32_t)(SDI_KEY) << 16) | (uint32_t)(data))
#define SDI_CFGR_OUTEN (1U << 10) /* CFGR[10] enable / CPBR[10] OUTSTA */
#define SDI_TDIVCFG_MASK 0x3U     /* CFGR[1:0] field mask / CPBR[1:0] TDIV */
#define SDI_TDIVCFG_DIV1 0x0U     /* 00: divided by 1 -> fast 1x, T = 125 ns */
#define SDI_TDIVCFG_DIV2 0x1U     /* 01: divided by 2 -> normal 2x (reset)   */

/* The word written by sdi_reset(): §2.4(1) only, i.e. slave output enabled.
 * Read the mask, not the value - CFGR[1:0] = 0b00 means TDIVCFG is *not*
 * committed, so the ÷1 in the shadow is inert and the part keeps its post-reset
 * time base. Committing ÷1 as well would take CFGR = 0x5AA50403 (mask [1:0] =
 * 0b11); that is deliberately NOT done here, because 0x5AA50400 to both 0x7E and
 * 0x7D is the pair validated on hardware. The waveform's fit to the fast-1x
 * windows (T = 125 ns, see sdi.pio) is therefore measured behaviour: the part
 * answers CPBR.TDIV = 0b11, which the manual lists as reserved. */
#define SDI_CONFIG_OUT_DIV1 (SDI_CFGR_OUTEN | SDI_TDIVCFG_DIV1)

// 9-bit Header Structure (Start bit + 7 Address bits + 1 R/W bit)
#define SDI_START_BIT 1
#define SDI_ADDRESS_BITS 7
#define SDI_MAX_ADDRESS ((1U << SDI_ADDRESS_BITS) - 1U)
#define SDI_HEADER_BITS 9
#define SDI_WORD_BITS 32
#define SDI_READ_FLAG 0U  /* host reads from the DM */
#define SDI_WRITE_FLAG 1U /* host writes to the DM */

/* Post-frame idle. The stop convention (§2.2) needs >= 10 times the time base in
 * 1x (>= 18 in 2x): 55 us covers both, and only has to be that long after a
 * write because a read frame is followed by the caller's own turn-around. */
#define INTER_WORD_DELAY 2
#define END_OF_WRITE_DELAY 55

static inline uint32_t make_sdi_header(uint8_t tgt, uint8_t mode)
{
    /* Callers must reject addresses wider than 7 bits: they would be shifted
     * into the start bit (see sdi_dm_read / sdi_dm_write). */
    return (SDI_START_BIT << 8) | ((uint32_t)tgt << 1) | (mode ? 1U : 0U);
}

/**
 * @brief Apply one configuration value using the §2.4 shadow/commit protocol.
 * Writes the value to SHDWCFGR and then the same word to CFGR to commit it. The
 * word must carry SDI_CFGR_OUTEN: a SHDWCFGR write replaces the whole shadow
 * value, so leaving bit 10 out would disable the slave output driver.
 * @param value Full 16-bit configuration word (see SDI_CONFIG_OUT_DIV1).
 */
static void sdi_write_config(const uint32_t value)
{
    sdiFrameWrite(SDI_SHDWCFGR, SDI_KEYED(value));
    lnDelayMs(2);
    sdiFrameWrite(SDI_CFGR, SDI_KEYED(value));
    lnDelayMs(2);
}

/**
 * @brief Reset the SDI interface (§2.4(3)) and enable the slave output (§2.4(1)).
 *
 * The wire idles high through the pad pull-up, so a self-timed low pulse of
 * more than 32 time bases resets the interface "regardless of the mode". The
 * word pair that follows (0x5AA50400 to 0x7E then 0x7D) is the sequence
 * validated on hardware and is kept byte-identical; it commits OUTEN only (see
 * SDI_CONFIG_OUT_DIV1), so the effective time base stays whatever the part comes
 * out of reset with. The waveform's fit to the fast-1x windows (T = 125 ns) is
 * therefore measured behaviour: the part answers CPBR.TDIV = 0b11, which the
 * manual lists as reserved.
 */
static void sdi_reset(uint32_t ms)
{
    sdiWireHoldLow(ms); // §2.4(3), holds the wire low

    sdi_write_config(SDI_CONFIG_OUT_DIV1); // §2.4(1), validated word pair
}

/**
 * @brief Human-readable name of a CPBR.TDIV field value.
 */
static const char *sdi_tdiv_name(const uint32_t tdiv)
{
    switch (tdiv)
    {
    case SDI_TDIVCFG_DIV1:
        return "div1/fast-1x";
    case SDI_TDIVCFG_DIV2:
        return "div2/normal-2x";
    default:
        /* Not a fault: the manual only documents 00/01 here, but a working
         * CH32V003 answers 0b11 (measured 2026-09-18 on a live link). CPBR is
         * informational - nothing here may drive a configuration write. */
        return "undocumented";
    }
}

/**
 * @brief Report the capability register (CPBR) so the negotiated mode is visible.
 * Measured on a live CH32V003 (2026-09-18): CPBR = 0x10403 gives VERSION = 1 and
 * OUTSTA = 1 exactly as the manual describes, but TDIV[1:0] = 0b11, which the
 * manual only lists as "reserved". The register is informational: report it,
 * never let it drive configuration. An all-ones answer means the slave is not
 * driving the line (OUTSTA = 0) - re-writing SHDWCFGR to "fix" a read is what
 * took the link down before.
 *
 * Single Logger call on purpose: Logger() formats into one shared static buffer
 * that the output path drains asynchronously, so two back-to-back calls can drop
 * the first line.
 */
static void sdi_log_config(const char *const tag)
{
    const uint32_t cpbr = sdiFrameRead(SDI_CPBR);
    if (cpbr == 0xFFFFFFFFUL)
    {
        Logger("SDI : CPBR %s = all ones (no slave output, OUTSTA=0)\n", tag);
        return;
    }
    Logger("SDI : CPBR %s = 0x%x (VERSION=0x%x, OUTSTA=%u, TDIV=0x%x/%s)\n", tag, (unsigned)cpbr,
           (unsigned)(cpbr >> 16), (unsigned)((cpbr >> 10) & 1U),
           (unsigned)(cpbr & SDI_TDIVCFG_MASK), sdi_tdiv_name(cpbr & SDI_TDIVCFG_MASK));
}

// ---------------------------------------------------------------------------
// Integrated SDI probe driver (stage 1: transport).
//
// Mirrors the RVSWD stack (bmp_rvTap_*.cpp / rvswd_template.h):
//   - bmp_gpio_pinmode(BMP_PINMODE_SDI) hands the debug pins to the platform's
//     SDI transport (PIO program upload on RP2040, sdi_pinmode_enter() on LN),
//   - these functions then drive the WCH DM (debug module) registers over the
//     platform's frame primitives (sdiFrameWrite / sdiFrameRead).
// ---------------------------------------------------------------------------
/**
 * @brief Enter SDI debug mode: put the pins in SDI mode, pulse NRST, reset and
 *        configure the SDI interface (slave output on) and report the mode the
 *        interface says it is in.
 *
 * Every session starts from the ÷2 normal-2x reset default (NRST and the §2.4(3)
 * hold-low both clear the interface configuration), which is not what the
 * platform waveform expects, so the mode is configured here rather than assumed.
 * The DM itself is not touched: riscv_debug.c::riscv_dm_init() activates it after
 * sdi_scan() attaches.
 * @return true
 */
bool LN_FAST_CODE sdi_dm_start()
{
    bmp_gpio_pinmode(BMP_PINMODE_SDI);
    if (pReset)
    {
        pReset->on(); // assert NRST
        lnDelayMs(20);
        pReset->off(); // release NRST
    }
    sdi_reset(20); // §2.4(3) hold-low, then §2.4(1) configuration

    /* Report what the interface thinks it is doing. Read-only: the validated
     * configuration above is never second-guessed on the strength of a read. */
    sdi_log_config("after unlock");
    return true;
}

/**
 * @brief Write one DM register over SDI.
 * @param adr Register address (7 bits).
 * @param val Value to write.
 * @return true
 */
bool LN_FAST_CODE sdi_dm_write(const uint8_t adr, const uint32_t val)
{
    if (adr > SDI_MAX_ADDRESS)
    {
        Logger("SDI : write to out-of-range address 0x%02x rejected\n", (unsigned)adr);
        return false;
    }
    sdiFrameWrite(adr, val);
    return true;
}

/**
 * @brief Read one DM register over SDI.
 * @param adr    Register address (7 bits; see SDI_MAX_ADDRESS).
 * @param output Receives the 32-bit register value.
 * @return false when the address cannot be encoded in a frame, or output is null.
 */
bool LN_FAST_CODE sdi_dm_read(const uint8_t adr, uint32_t *const output)
{
    if (!output)
        return false;
    if (adr > SDI_MAX_ADDRESS)
    {
        Logger("SDI : read of out-of-range address 0x%02x rejected\n", (unsigned)adr);
        return false;
    }
    *output = sdiFrameRead(adr);
    return true;
}

extern "C"
{
    /** @brief C entry point: enter SDI debug mode (mirror of bmp_rv_dm_reset_c). */
    bool bmp_sdi_dm_reset_c()
    {
        return sdi_dm_start();
    }
    /** @brief C entry point: SDI DM write (mirror of bmp_rv_dm_write_c). */
    bool bmp_sdi_dm_write_c(const uint8_t adr, const uint32_t value)
    {
        return sdi_dm_write(adr, value);
    }
    /** @brief C entry point: SDI DM read (mirror of bmp_rv_dm_read_c). */
    bool bmp_sdi_dm_read_c(const uint8_t adr, uint32_t *const value)
    {
        return sdi_dm_read(adr, value);
    }
}

/* Bounded retry budget for DMI accesses, mirroring RVSWD_DMI_MAX_ATTEMPTS in
 * rvswd_template.h. SDI transfers do not return a per-access DMI status word,
 * so the retry loop only guards against the transport being momentarily
 * unresponsive (e.g. right after an unlock/reset). */
#define SDI_DMI_MAX_ATTEMPTS 4U
#define RV_DMI_SUCCESS 0U
#define RV_DMI_FAILURE 2U

static bool ch32_sdi_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    for (uint32_t attempt = 0U; attempt < SDI_DMI_MAX_ATTEMPTS; ++attempt)
    {
        if (sdi_dm_read((uint8_t)address, value))
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    dmi->fault = RV_DMI_FAILURE;
    return false;
}

static bool ch32_sdi_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    for (uint32_t attempt = 0U; attempt < SDI_DMI_MAX_ATTEMPTS; ++attempt)
    {
        if (sdi_dm_write((uint8_t)address, value))
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    dmi->fault = RV_DMI_FAILURE;
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
    sdi_dm_start();
    target_list_free();

    uint32_t status = 0;
    for (int retry = 0; retry < 5; retry++)
    {
        (void)sdi_dm_read(DMSTATUS, &status);
        if (status != 0xFFFFFFFFUL) break;
        lnDelayMs(1);
    }

    if (status == 0xFFFFFFFFUL)
    {
        Logger("SDI : no target responding (DMSTATUS=0x%x)\n", (unsigned)status);
        return false;
    }
    /* Report what the DM says about itself. One Logger call per path: Logger()
     * formats into a single shared buffer that is drained asynchronously, so a
     * second back-to-back call can drop the first line. The all-ones gate above
     * stays deliberately loose (it only means "nobody is driving the wire"): the
     * authoritative version check is riscv_dmi_init() -> riscv_dm_init(), which
     * decodes dmstatus itself and rejects anything but v0.13/v1.0. */
    if ((status & RV_STATUS_VERSION_MASK) == SDI_DMSTATUS_VERSION_0_13)
        Logger("SDI : found target DMSTATUS=0x%x (version=%u v0.13, authenticated=%u), attaching RISC-V DM\n",
               (unsigned)status, (unsigned)SDI_DMSTATUS_VERSION_0_13,
               (unsigned)((status & SDI_DMSTATUS_AUTHENTICATED) ? 1U : 0U));
    else
        Logger("SDI : WARNING DMSTATUS=0x%x reports version=%u where v0.13 (%u) is expected; "
               "riscv_dm_init() will reject it\n",
               (unsigned)status, (unsigned)(status & RV_STATUS_VERSION_MASK),
               (unsigned)SDI_DMSTATUS_VERSION_0_13);

    riscv_dmi_s *dmi = new riscv_dmi_s;
    if (!dmi)
    { /* allocation failed: heap exhaustion */
        Logger("SDI : dmi allocation failed in %s\n", __func__);
        return false;
    }
    memset(dmi, 0, sizeof(*dmi));
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    /* SDI frames carry a 7-bit DMI address field (start+7addr+RW header, see
     * make_sdi_header), matching wchlink_riscv_dtm.c. The tap never reads this
     * field (fixed-frame driver), but it must describe the bus correctly. */
    dmi->address_width = 7U;
    dmi->read = ch32_sdi_dmi_read;
    dmi->write = ch32_sdi_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}
