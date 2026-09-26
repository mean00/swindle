/**
 * @file bmp_sdiTap_ln_bitbang_riscv_extra.cpp
 * @brief WCH SDI tap for the LN platform: CPU-timed (bit-bang) transport.
 *
 * The sibling file bmp_sdiTap_ln_riscv_extra.cpp drives the wire with a timer channel plus
 * DMA: a PWM carrier makes the cells of a write frame, the same channel clocked
 * by the CPU makes a read frame's, and two more channels fire DMA transfers into
 * the level shifter's direction pin. This variant makes *both* halves of a frame
 * with the CPU and touches neither a timer channel nor a DMA engine.
 *
 * Why a second transport exists: the DMA path can wedge the probe. Arming the DIR
 * channel pair for a read frame (SdiTimer::armDirFrame()) happens with interrupts
 * masked, and lnDMA::beginTransfer() takes its channel's mutex there. When that
 * mutex is already held by another task, the call parks in
 * xQueueTakeMutexRecursive() - inside the masked region - and the task that would
 * release it can never run: the probe stops answering, with the backtrace parked
 * in armDirFrame() -> beginTransfer() -> lnMutex::lock() ->
 * vTaskPlaceOnEventList(). Nothing in this file can block that way (one shared
 * resource, PB8/PC3, owned for the whole session, and no locks at all), so the
 * mask it uses cannot wedge anything either.
 *
 * What it costs: the clock is a counted NOP loop, and the pad accesses that make
 * the edges are bus time the loops cannot account for. Both are measured at mode
 * entry and taken out of the loop counts, so a cell comes out at the 896 ns the
 * waveform asks for instead of 896 ns plus four pad accesses. What is left is
 * granularity: one iteration is ~30 ns, i.e. 3% of a cell and 13% of a LOW, so
 * the LOW widths land in the right window of §2.2 rather than on the exact
 * nanosecond, and a response cell's turn-arounds cost it about one iteration
 * each. The cell it actually produces is measured and printed at mode entry
 * (sdiLogMode()) - this is the transport that cannot wedge, not the one with the
 * hardware path's exact edges.
 *
 * The pad's role inside a frame is the hardware path's: push-pull while the probe
 * drives both levels (a write frame, and the write-like header of a read frame -
 * a release through the level shifter's pull-up is not a defined edge there),
 * open drain for the response cells, where releasing the wire *is* the point.
 *
 * Everything above the wire is the shared protocol layer
 * (../../template/sdi_template.h: DM/configuration registers, the §2.4
 * shadow/commit protocol, the frame format, CPBR diagnostics, the DMI glue and
 * the RISC-V attach), included here exactly as it is there, so only the transport
 * differs between the two LN taps and the other platforms.
 */

// -------------------------------------------------- //
#include "lnGPIO.h"
#include "stdint.h"
#include "esprit.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_tap_ln.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
#include <cstring>
/* The SDI seam (sdi_pinmode_enter/leave, the 'mon sdi_wire' knobs): the one set of
 * declarations every non-SDI file shares with this one. */
#include "bmp_riscv_extra.h"

// The blackmagic RISC-V target framework (riscv_debug.h) is needed for the
// stage-2 attach below, exactly as in the timer/DMA tap: we hand
// riscv_dmi_init() an SDI-backed riscv_dmi_s, mirroring what
// rvswd_template.h::rvswd_scan() does over RVSWD.
extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
extern "C" void target_list_free(void);

    /* Per-frame trace switch: the same opt-in flag the memory path's
     * instrumentation uses (`mon memlog on|off`, bmp_mem_log_enabled).
     *
     * A frame trace is written once per DMI frame, i.e. *inside* the GDB command
     * that made the frame. When the host streams that command's reply (a `g`
     * register read, an `m` byte read), the trace goes out on the log CDC while
     * the reply goes out on the GDB CDC, and on this build the two are observed
     * to cross over: the log line lands in the middle of the reply packet (the
     * host reports "Invalid hex digit 32", a bad checksum and then a dropped
     * packet) and reply bytes land inside the log line. Keeping the hot path
     * quiet keeps both clean - the one-shot lines (mode entry, mode exit) are
     * emitted outside the streamed replies, so they stay on by default. See
     * riscv_fault.md for the capture. */
    extern "C" bool bmp_mem_log_enabled;
    /* SDI wire timing overrides ('mon sdi_wire', bmp_set_sdi_wire_c()): the four
     * numbers, in ns, that §2.2 fixes - the cell, the LOW that carries a 1, the
     * LOW that carries a 0 and the response sample point - and whether the counts
     * are trimmed onto the cell at all. A zero means "the default this file
     * defines", which is also the power-on state. Declared in bmp_riscv_extra.h,
     * defined by bmp_riscv_extra_sdi_c.cpp ('mon sdi_wire' sets them). */

    /* SDI data pin: the same physical wire the SWD/RVSWD taps use, i.e. the
     * SWDIO line _mapping[TSWDIO_PIN] (PB8 on both packages).
     *
     * The pad has to change role inside a frame (it drives both levels while the
     * probe writes, and it makes the read clock while the target drives), and the
     * level shifter direction has to follow, so both go through rSWDIO
     * (bmp_tap_ln.cpp): _fast is the pad's BOP set/clear word and _fastdir the
     * direction pin - one bus access per edge or turn-around, and the unit this
     * transport's timing is measured in. */
#define SDI_DATA_PIN _mapping[TSWDIO_PIN]

    // ---------------------------------------------------------------------------
    // Platform transport hooks consumed by the shared protocol layer.
    //
    // The same three hooks the timer/DMA tap supplies, and macros for the same
    // reason: the protocol layer only ever calls them once per frame (never
    // inside a bit cell), so the indirection costs nothing and the frame-level
    // code expands to byte-identical source on every platform.
    // ---------------------------------------------------------------------------
    static void sdiWriteFrame(const uint8_t adr, const uint32_t data);
    static uint32_t sdiReadFrame(const uint8_t adr);

#define sdiFrameWrite(adr, val) sdiWriteFrame((adr), (val))
#define sdiFrameRead(adr) sdiReadFrame(adr)
    /* §2.4(3): hold the wire low for @p ms, then let it float high again. The
     * pulse has to come from the pad itself - DIR back to the probe, ODR 0 sinks
     * it, and releasing ODR lets the level shifter's pull-up float the wire
     * high. */
#define sdiWireHoldLow(ms)                                 \
    do                                                     \
    {                                                      \
        rSWDIO->_fastdir.on(); /* DIR = probe */           \
        rSWDIO->_fast.off();   /* Drive LOW */             \
        lnDelayMs(ms);                                     \
        rSWDIO->_fast.on();    /* Release to float HIGH */ \
    } while (0)

    /* The shared protocol layer (sdi_dm_start/write/read, bmp_sdi_dm_*_c,
     * ch32_sdi_dmi_read/write, sdi_scan). Requires: bmp_pinmode.h, pReset,
     * rSWDIO/bmp_tap_ln.h, Logger/lnDelayMs/lnDelayUs, <cstring> and the
     * blackmagic RISC-V target framework, all included above. */
#include "sdi_template.h"
}

// ---------------------------------------------------------------------------
// Bit cell (§2.2): one cell per bit, a 1 is a LOW of 1/4 cell (250 ns at the cell
// below) and a 0 a LOW of 3/4 cell (750 ns), the wire high for the rest of the cell,
// and the target's response is sampled 11/16 of a cell (687 ns) after the boundary.
// The fractions are the ones the RP2040 timer/DMA tap uses (sdi.pio), which was
// brought up on a 896 ns cell; BMP 103 scaled this tap's cell to 1000 ns because the
// trim was landing the wire at 930 ns and ~1 read in 30 came back wrong on a
// CH32V003 - see SDI_TBIT_NS below and riscv_fault.md §12.9.
//
// The same fractions the timer/DMA tap uses, in nanoseconds instead of timer
// ticks: this transport's clock is a counted NOP loop, so the numbers below are
// the waveform and sdiConfigureTiming() turns them into loop counts, measured on
// the part rather than assumed.
// ---------------------------------------------------------------------------
#define SDI_TBIT_NS 1000u     /* one cell */
/* BMP 103 (riscv_fault.md §12.9): these four numbers were 896/224/672/616 - the 1x time
 * base the RP2040 tap was brought up with - and with them the *trim* lands the wire on
 * cell 930 / LOW1 4+11 / LOW0 11+4 (sdiConfigureTiming() divides every count by
 * cellNs/tbitNs and the measured frame cell is ~1135 ns), where one 4-byte GDB read in
 * thirty comes back wrong and silent, holding whatever DATA0/a1 last held. The same
 * waveform scaled to a 1000 ns cell lands on cell 1010 / LOW1 4+13 / LOW0 13+4, and the
 * loss disappears: alternating phases of one session, 51 wrong words of 1856 at 896 ns
 * against 0 of 1856 at 1000 ns. What the log shows differing is the cell and the two
 * 13-loop phases (the data-0 LOW and the HIGH after a 1); the 1's LOW sits at its 4-loop
 * floor either way, and the response sample point makes no difference on its own
 * (1000/250/750/616: 0 of 384; 896/224/672/687: 8 of 384).
 *
 * Verified on the flashed image (riscv_fault.md §12.9.6, 2026-09-20): at this cell and with
 * BMP 103's confirmed read on, 384-word sweeps came back 384/384 right with 0 disagreeing
 * windows, 0 elements settled and 0 faulted passes, and a 13 KB `compare-sections` matched
 * every section; the 896 ns cell, with the same confirm on, still lost 27 of 384 words. */
/* §2.2's fractions, and what the counts are placed at. sdi.txt §2.2 states a cell's
 * LOW width as a multiple of the slave's time base T (a data 1 is a LOW in (T, 2T),
 * a data 0 one in (4T, 32T), a stop bit a HIGH of 10T) rather than as an absolute
 * time, and the tap's 896 ns cell with the two numbers below is the waveform the
 * RP2040 transport was brought up with - 224/896 and 672/896 are 1T and 3T of a 4T
 * cell - and the one a CH32V003 attaches to.
 *
 * What has to survive is that the *proportions* reach the wire. A frame cell spends
 * a per-cell constant outside its counted loops which is not a proportion of
 * anything: 300 ns measured on this GD32F303 (sdiConfigureTiming()), a third of a
 * cell, and *more* than the 224 ns a data 1's LOW is made of. There are two ways to
 * fit it into a cell: hold the cell at 896 ns and let the trim compress both LOWs by
 * ~30% to get it (what this file did, and what put a 1's LOW at 168 ns of the 224 it
 * was asked for), or place the LOWs at the times below and let the cell come out
 * ~300 ns longer than 896 ns. The measurements say the second: a 1's LOW shorter
 * than about 200 ns attaches monotonically worse on this target (2 loops on the
 * wire: no attach at all; 3: one session in three; 4-5: every session of every
 * boot), so the LOW is the last thing to trim to make a cell fit - a cell 1.3x
 * slower is a waveform the target decodes, a LOW a third short is not.
 *
 * BMP 103 measured what those two ways actually come to on this part - see SDI_TBIT_NS above:
 * the *trimmed* phases are what has to keep the margin, and 896/224/672/616 did not (930 ns
 * cell, 11-loop phases, ~1 read in 30 wrong) while 1000/250/750/687 does (1010 ns cell,
 * 13-loop phases, 0 of 1856).
 *
 * `mon sdi_wire` overrides the four numbers below (see riscv_fault.md §12). */
#define SDI_LOW1_NS 250u      /* 1/4 cell: the LOW that carries a 1 */
#define SDI_LOW0_NS 750u      /* 3/4 cell: the LOW that carries a 0 */
#define SDI_RX_SAMPLE_NS 687u /* 11/16 cell: where a response bit is sampled */

/* Fallback loop cost, in tenths of a nanosecond, until the measurement has run:
 * the loop is three instructions on this part and one iteration measures ~42 ns
 * at 96 MHz, which is the waveform's own figure (sdiLoopNsX10) and the one every
 * phase count here is consistent with. A fallback of 12.5 ns - the per-bit figure
 * bmp_set_frequency_c() calibrates SWD from - is three times too fast for this
 * loop: with it the phases come out ten times too long, the target sees no clock
 * and the session is dead. It only stands in when the measurement is rejected
 * (see SDI_LOOP_NS_X10_MIN), which is the case where being approximately right
 * matters. */
#define SDI_LOOP_NS_X10_DEFAULT 422u

/* The cell the counts are trimmed against (see sdiConfigureTiming()): 200 cells
 * is a ~0.18 ms burst, which the microsecond timer resolves to 0.5% of a cell, and
 * three passes where two land the cell inside the one iteration a count can
 * resolve - the spare costs half a millisecond at mode entry and bounds the loop
 * whatever the part does.
 *
 * 200 and not the ~1000 cells a longer burst would buy: the burst is masked, and
 * a masked burst cannot be longer than the microsecond timer can carry on its
 * own. One build of this file masked 1000 of them (0.9 ms) and lnGetUs() came
 * back reporting a *negative* cell - "cell 4294967265" in the mode entry log -
 * because the interrupt that carries the timer's high part cannot run inside the
 * mask. The same happened to the 1000-cell probe burst in the same log
 * ("probe 4294965846"), while the empty burst at 1000 cells of two accesses
 * (~0.1 ms) measured exactly right. So: every masked burst in this file is a
 * fraction of a millisecond - the frames mask for 45 us a frame - and the wrap
 * point of the timer is what the cell count here is really sized against. */
#define SDI_TRIM_CELLS 200u
#define SDI_TRIM_PASSES 3u

/* How many bursts a measurement keeps the smallest of, and what a cell can
 * plausibly be. The bursts are timed with the cycle counter and the reading is
 * used as a *divisor* (SDI_TBIT_NS / cellNs scales every count), so a burst that
 * measured wrongly does not degrade the waveform, it destroys it: a write cell
 * read as 10.7 us and up trims every count to zero, the frames then make no LOW
 * at all and the target sees no clock - "CPBR after unlock = all ones (no slave
 * output)", DMSTATUS=0xffffffff. One boot of this file did exactly that: with the
 * timing bursts masked, the code looking for the timer was re-rolled, the two
 * lnGetUs() calls the burst is bracketed by were folded into the wrong order and
 * "one cell measures 4294243" came out - the wrapped difference of an end read
 * that landed before its start. That is what sdiTimeUs()'s asm barrier is for, so
 * the bursts are masked again - an interrupt inside one is not only noise, it is
 * *proportional* noise (see sdiMeasureProbeCellNs()) - and on top of that: the
 * smallest of SDI_CAL_PASSES bursts is kept (interference only ever makes a burst
 * longer, never shorter, so the minimum is the reading closest to what the CPU
 * alone costs), and a cell outside SDI_CELL_MIN_NS..SDI_CELL_MAX_NS is not used to
 * trim at all - it is a broken measurement, and the counts the model built stand.
 * See riscv_fault.md. */
#define SDI_CAL_PASSES 3u
#define SDI_CELL_MIN_NS 400u
#define SDI_CELL_MAX_NS 4000u

/* The probe burst of the two-point calibration: 100 iterations a half over 25
 * cells, i.e. ~0.21 ms a burst. Sized like SDI_TRIM_CELLS and for the same reason:
 * the burst is masked, so it has to stay inside what the microsecond timer carries
 * across a mask. 25 cells still resolve the timer to 20 ns a cell, which the 200
 * iterations the loop cost is divided by turn into 0.1 ns. */
#define SDI_PROBE_CELLS 25u
#define SDI_PROBE_LOOPS 100u

/* What a loop iteration can plausibly cost, in tenths of a nanosecond. A reading
 * outside it is a broken measurement and the value already in sdiLoopNsX10 stands:
 * unlike the cell, which the trim corrects against the frames, nothing downstream
 * would notice a loop cost read as almost nothing - every phase count is
 * (ns - accesses * accessNs) / loop, so a small loop turns the counts into
 * multi-second phases and the frames into no clock at all. The window is wide
 * (3 instructions to a few hundred nanoseconds an iteration) because this is the
 * only thing standing between a wrapped reading and a dead session. */
#define SDI_LOOP_NS_X10_MIN 50u
#define SDI_LOOP_NS_X10_MAX 5000u

/* Floors for the response phases, in loops. A cell is counts * loop + a per-cell
 * constant (the comment in sdiConfigureTiming() calls it 157 ns on a GD32F303 at
 * 96 MHz), and the trim scales the counts: a count that came out on its floor has
 * to stay on it, since the constant is exactly what the factor cannot represent.
 * sdiRxLowLoops + sdiRxMidLoops is where the response bit is sampled, so a phase
 * the trim took a loop short moves the sample point 41 ns - and the response
 * window is only a handful of loops wide. This is not theoretical: with
 * sdiRxMidLoops trimmed to 1 the sample lands ~380 ns early, riscv_dm_init()
 * reads a DMSTATUS whose version field is not 2 (5 and 10 were seen, against the
 * expected 2) and refuses the target, i.e. every session of that boot fails to
 * attach - see riscv_fault.md. */
#define SDI_RX_LOW_MIN_LOOPS 2u
#define SDI_RX_MID_MIN_LOOPS 3u
#define SDI_RX_TAIL_MIN_LOOPS 1u

/* What the measurement found, and the loop counts derived from it. The counts
 * stay zero until sdiConfigureTiming() runs, which every frame is behind: frames
 * only start once bmp_gpio_pinmode(BMP_PINMODE_SDI) has called
 * sdi_pinmode_enter() (see sdi_dm_start()). */
static uint32_t sdiLoopNsX10 = SDI_LOOP_NS_X10_DEFAULT; /* one loop iteration */
static uint32_t sdiAccessNs = 0;                        /* one pad access */
static uint32_t sdiLow1Loops = 0, sdiHigh1Loops = 0;    /* a cell whose bit is 1 */
static uint32_t sdiLow0Loops = 0, sdiHigh0Loops = 0;    /* a cell whose bit is 0 */
static uint32_t sdiRxLowLoops = 0, sdiRxMidLoops = 0, sdiRxTailLoops = 0;

/* What the counts above made before they were trimmed onto the cell, for the
 * mode entry log: the trim factor is how far the model was off. */
static uint32_t sdiCellUntrimmedNs = 0;

/* What one loop iteration costs *in a frame* (tenths of a ns) and what a frame
 * cell costs outside its counted loops (ns): the two numbers the counts are
 * finally placed with, both measured on the frame path by the trim loop (see
 * sdiConfigureTiming()). Zero until that measurement lands, which is also the
 * "use the model as it is" state - a knob-driven 'mon sdi_wire ... notrim' run, or
 * a boot whose two readings do not bracket the constant, keeps the counts the
 * model built and behaves exactly as this file did before they existed. */
static uint32_t sdiFrameLoopNsX10 = 0, sdiCellFixedNs = 0;

/* The two ends of the loop-cost measurement, in ns per cell, for the mode entry
 * log: what the pad accesses and the loop entries cost (empty), and what they
 * cost with 200 iterations a cell added (probe). The difference between the two
 * divided by 200 is sdiLoopNsX10, so a busy window shows up in these two numbers
 * first - and proportionally, which no minimum can take out. */
static uint32_t sdiEmptyCellNs = 0, sdiProbeCellNs = 0;

/* SDI owns PB8/PC3 for the whole session; the timing is calibrated once. */
static bool sdiMode = false;
static bool sdiTimingDone = false;

/* The wire timing in one value, so "has 'mon sdi_wire' changed it?" is one
 * comparison per field: sdiWireRead() is what the knob holds now, sdiWireUsed
 * what the last calibration read of it, and a difference between the two is what
 * makes sdi_pinmode_enter() calibrate again. Both start at zero, which is also
 * the knob's "leave it at the default" state - so a build nobody runs the knob
 * on calibrates exactly as it did before the knob existed. */
struct SdiWire
{
    uint32_t tbitNs, low1Ns, low0Ns, sampleNs;
    bool noTrim;
};
static SdiWire sdiWireUsed = {0u, 0u, 0u, 0u, false};

/** @brief What 'mon sdi_wire' holds now, in ns (0 = this file's default). */
static SdiWire sdiWireRead()
{
    SdiWire wire;
    wire.tbitNs = bmp_sdi_wire_tbit_ns;
    wire.low1Ns = bmp_sdi_wire_low1_ns;
    wire.low0Ns = bmp_sdi_wire_low0_ns;
    wire.sampleNs = bmp_sdi_wire_sample_ns;
    wire.noTrim = bmp_sdi_wire_no_trim;
    return wire;
}

/** @brief Has 'mon sdi_wire' changed the timing since the last calibration? */
static bool sdiWireChanged()
{
    const SdiWire wire = sdiWireRead();
    return wire.tbitNs != sdiWireUsed.tbitNs || wire.low1Ns != sdiWireUsed.low1Ns ||
           wire.low0Ns != sdiWireUsed.low0Ns || wire.sampleNs != sdiWireUsed.sampleNs ||
           wire.noTrim != sdiWireUsed.noTrim;
}

/** @brief The knob's @p ns, or @p dflt when it is zero, i.e. not overridden. */
static uint32_t sdiWireValue(const uint32_t ns, const uint32_t dflt)
{
    return ns ? ns : dflt;
}

// ---------------------------------------------------------------------------
// The wire. _fast/_fastdir are BOP registers, i.e. one store per edge with no
// read-modify-write, which is what keeps a CPU made cell as short as it is - and
// what sdiAccessNs measures.
// ---------------------------------------------------------------------------
/** @brief Level shifter -> probe: the pad's level reaches the target. */
static inline LN_ALWAYS_INLINE void sdiDirProbe()
{
    rSWDIO->_fastdir.on();
}
/** @brief Level shifter -> target: the probe lets go of the wire. */
static inline LN_ALWAYS_INLINE void sdiDirTarget()
{
    rSWDIO->_fastdir.off();
}
/** @brief Sink the wire (a LOW in every pad mode this file uses). */
static inline LN_ALWAYS_INLINE void sdiPadLow()
{
    rSWDIO->_fast.off();
}
/** @brief Release the wire: driven HIGH in push-pull, floating HIGH in open drain. */
static inline LN_ALWAYS_INLINE void sdiPadRelease()
{
    rSWDIO->_fast.on();
}
/** @brief The wire's level, without lnDigitalRead()'s port lookup and call. */
static inline LN_ALWAYS_INLINE bool sdiReadPad()
{
    return rSWDIO->_fast.read();
}
/** @brief The probe drives both levels: a write cell, or a read header cell. */
static inline LN_ALWAYS_INLINE void sdiPadDriven()
{
    lnPinMode(SDI_DATA_PIN, lnOUTPUT, SWD_IO_SPEED);
}
/** @brief Only the LOW is driven; the HIGH is a release, so the target can drive. */
static inline LN_ALWAYS_INLINE void sdiPadOpenDrain()
{
    lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, 1);
}
/**
 * @brief The idle wire: released, shifter pointing at the probe, open drain.
 *
 * What every frame starts and ends in, and the state sdi_pinmode_enter() leaves
 * the pad in - including after the timing measurement, which parks it differently
 * for the duration (see there).
 */
static void sdiPark()
{
    sdiPadRelease();
    sdiDirProbe();
    sdiPadOpenDrain();
}

// ---------------------------------------------------------------------------
// The clock: a counted NOP loop.
//
// Not swait(): that one walks swd_delay_cnt, i.e. the wait state the *user* picked
// for SWD ("mon freq"), which has nothing to do with an SDI bit cell. The counts
// here come from the cell's fractions in nanoseconds (sdiConfigureTiming()), so
// the waveform survives a different CPU clock without a table of per-part
// constants.
// ---------------------------------------------------------------------------
/**
 * @brief Walk @p loops nop iterations.
 *
 * volatile so the loop cannot be dropped as having no effect, and a counted
 * down-loop so that every site the compiler emits has the same shape: the loop
 * cost measured at mode entry is then the cost the frames pay, down to the
 * instruction.
 *
 * The test is at the top, so 0 is 0 iterations. That is not a corner case here:
 * sdiPhaseLoops() returns 0 whenever a phase's own pad accesses already cover it,
 * and sdiMeasureCellNs(0, 0, ..) is a phase with no loops *by design* - it is how
 * the empty cell is measured. A down-loop that ran its body first
 * (do/while (--loops)) has no zero: it would wrap to UINT32_MAX and count down
 * from four billion iterations of an 896 ns cell.
 */
static inline LN_ALWAYS_INLINE void sdiLoop(uint32_t loops)
{
    while (loops--)
    {
        __asm__ __volatile__("nop");
    }
}

/**
 * @brief Make one bit cell: sink the wire for @p lowLoops iterations, release it
 *        for @p highLoops.
 *
 * A cell is a fixed time whose *LOW width* carries the bit value (§2.2), so only
 * the two halves have to be counted - which is what makes a loop enough to be the
 * clock. The pad access that makes each edge lands inside the half it belongs to;
 * that is what sdiPhaseLoops() takes out of the counts. The CALLER owns the pad
 * mode and the shifter direction: they are per frame or per phase, not per cell.
 */
static inline LN_ALWAYS_INLINE void sdiSendCell(const uint32_t lowLoops, const uint32_t highLoops)
{
    sdiPadLow();
    sdiLoop(lowLoops);
    sdiPadRelease();
    sdiLoop(highLoops);
}

/** @brief Make one bit cell whose bit value is @p bit: 1 is a short LOW, 0 a long one. */
static inline LN_ALWAYS_INLINE void sdiSendBit(const uint32_t bit)
{
    if (bit)
        sdiSendCell(sdiLow1Loops, sdiHigh1Loops);
    else
        sdiSendCell(sdiLow0Loops, sdiHigh0Loops);
}

// ---------------------------------------------------------------------------
// Timing: what this CPU does, measured once at SDI mode entry.
// ---------------------------------------------------------------------------
/**
 * @brief Loop iterations for a phase of @p ns that carries @p accesses pad
 *        accesses of its own.
 *
 * A pad access (a BOP store making an edge, or the load that samples the pad)
 * lands *inside* the phase it makes, so a phase would be @p ns plus its own bus
 * time without this: the accesses and the loop's own entry are taken out of the
 * count, which is what keeps the fractions the fractions *of the cell* rather
 * than of the loops. Rounds up - the LOW times are documented minima (§2.2) - and
 * returns 0 when the accesses already cover the phase, i.e. for a half the pad
 * accesses pay for on their own.
 */
static uint32_t sdiPhaseLoops(const uint32_t ns, const uint32_t accesses)
{
    const uint32_t spent = accesses * sdiAccessNs + sdiLoopNsX10 / 10u;
    if (ns <= spent)
        return 0u;
    return ((ns - spent) * 10u + sdiLoopNsX10 - 1u) / sdiLoopNsX10;
}

/**
 * @brief Measure one cell, @p cells times over, in ns.
 *
 * Only the pad is touched, so this is the code path the frames use: it measures
 * the waveform, not a loop. Zero loops is the "cell" the two pad accesses make on
 * their own, which is how sdiConfigureTiming() separates the loop cost from the
 * access cost. 1000 cells are ~1 ms, short enough for the microsecond timer to
 * still resolve a single cell to 0.1%.
 *
 * Only the CPU is timed, so the pad is parked off the wire for the whole
 * calibration (sdi_pinmode_enter()): the store that makes an edge still costs what
 * it costs, and the burst that measures it reaches nobody. The zero-loop cell is
 * taken through sdiMeasureEmptyCellNs() instead, which keeps the smallest of
 * SDI_CAL_PASSES bursts of it.
 */
/**
 * @brief A microsecond the optimizer may neither move nor share.
 *
 * The two reads that bracket a burst are equal by construction - lnGetUs() takes
 * no argument - and one build of this file has been seen to fold them into the
 * wrong order while looking for the timer (see SDI_CELL_MIN_NS): a burst that
 * costs a millisecond came back as "one cell measures 4294243", and the divisor
 * that made turned every count to zero. The asm is a barrier neither call can
 * cross, so each read is taken where it is written.
 */
static inline LN_ALWAYS_INLINE uint32_t sdiTimeUs()
{
    __asm__ __volatile__("" ::: "memory");
    return lnGetUs();
}

static uint32_t sdiMeasureCellNs(const uint32_t lowLoops, const uint32_t highLoops, const uint32_t cells)
{
    const uint32_t startUs = sdiTimeUs();
    for (uint32_t i = 0; i < cells; i++)
        sdiSendCell(lowLoops, highLoops);
    return (uint32_t)(((uint64_t)(sdiTimeUs() - startUs) * 1000u) / cells);
}

/**
 * @brief The cell that has no loops in it at all, @p cells times over, in ns:
 *        the smallest of SDI_CAL_PASSES bursts, with interrupts masked.
 *
 * Two pad accesses and two loop entries, ~0.2 ms of counted cells, and the one
 * *absolute* reading in this calibration: sdiAccessNs comes straight out of it,
 * while the loop cost beside it is only a difference between two bursts of the
 * same length, where an interrupt adds about the same time to both and cancels.
 * So this is the measurement an ISR changes the meaning of: the same image
 * measured 11 ns and 191 ns of access on two consecutive boots with interrupts
 * on, against the ~42 ns a BOP store costs here (one loop iteration, and the
 * figure the pre-fix boots' phase counts are consistent with), and every phase
 * count is `(ns - accesses * sdiAccessNs) / loop`. At 113 ns the response phase
 * collapses, riscv_dm_init() reads a DMSTATUS whose version field is not 2 and
 * refuses the target, i.e. no session of that boot can attach - riscv_fault.md
 * has those measurements. This burst is short enough to mask (unlike the two
 * millisecond bursts, which are not), and the smallest of the passes is kept on
 * top of that: an interrupt only ever adds time to a burst. The pad is parked
 * off the wire for it (sdi_pinmode_enter()), so the burst reaches nothing but
 * the timer either way.
 */
static uint32_t sdiMeasureEmptyCellNs(const uint32_t cells)
{
    uint32_t best = 0u;
    lnNoInterrupt();
    for (uint32_t pass = 0; pass < SDI_CAL_PASSES; pass++)
    {
        const uint32_t ns = sdiMeasureCellNs(0u, 0u, cells);
        if (!best || ns < best)
            best = ns;
    }
    lnInterrupts();
    return best;
}

/**
 * @brief The probe cell, @p cells times over, in ns: the smallest of
 *        SDI_CAL_PASSES bursts, masked.
 *
 * sdiMeasureEmptyCellNs()'s twin - this is the other end of the two-point
 * calibration, and the difference between the two is what sdiLoopNsX10 is. The
 * burst is SDI_PROBE_LOOPS iterations a half over SDI_PROBE_CELLS cells (~0.21 ms)
 * precisely so that it *can* be masked: an interrupt inside it is reported as cell
 * time, and because the burst is 200 iterations a cell longer than the one it is
 * differenced against, that time is divided by 200 and lands in the loop cost
 * whole - and every phase count is built from that. Interference is a *fraction of
 * the window*, so the minimum of the passes cannot take it out: it hits the long
 * burst and not the short one, and it scales with however busy the window was.
 *
 * That is the shape of the defect this function was written for: one boot of this
 * file measured 50.1 ns of loop and 47.4 ns on another, against the ~42 ns the
 * phases are built from, with the empty burst beside them reading 4 and 7 ns of
 * access instead of 10-11. The counts those pairs produced moved the LOW and the
 * sample a fifth of a cell, riscv_dm_init() found a DMSTATUS with no slave output
 * behind it and no session of that boot could attach; the milder windows land the
 * fractions near enough to attach and corrupt a frame every few thousand (see
 * riscv_fault.md). Masking it costs the USB stack 0.21 ms once per mode entry -
 * the frames mask for 45 us a frame and the write cell below for 0.18 ms a burst -
 * and the asm barrier sdiTimeUs() is bracketed by is what makes masking safe
 * again (see SDI_CELL_MIN_NS).
 */
static uint32_t sdiMeasureProbeCellNs(const uint32_t loops, const uint32_t cells)
{
    uint32_t best = 0u;
    lnNoInterrupt();
    for (uint32_t pass = 0; pass < SDI_CAL_PASSES; pass++)
    {
        const uint32_t ns = sdiMeasureCellNs(loops, loops, cells);
        if (!best || ns < best)
            best = ns;
    }
    lnInterrupts();
    return best;
}

/**
 * @brief Measure the cell a frame bit makes, @p cells times over, in ns.
 *
 * sdiMeasureCellNs() takes its counts as arguments, so the compiler hoists the
 * loads out of the burst and what it times is the loop alone - which is what the
 * two-point calibration wants, and exactly what the frames do not pay: they read
 * the counts out of the statics for every cell, choose the cell at run time and
 * count their bits in an outer loop. This runs sdiSendBit(), the code the frames
 * run, cell by cell, so it times a cell in a frame. The bit value alternates -
 * the frames' is a run-time value too, and both cell shapes are the same number
 * of iterations (LOW1+HIGH1 = LOW0+HIGH0), so what comes out is one cell. Like the
 * calibration, it runs with the pad parked off the wire: what is measured is the
 * CPU's cell either way.
 *
 * The smallest of SDI_CAL_PASSES bursts, and masked per burst: the measurement is
 * the *divisor* every count is trimmed by (SDI_TBIT_NS / cellNs), so a window that
 * inflated it would not blur the waveform but shrink it - and an interrupt inside a
 * 0.18 ms burst is a fraction of that burst, which no minimum can take out (see
 * sdiMeasureProbeCellNs()). 0.18 ms of mask per pass, half a millisecond per mode
 * entry, against the 45 us a frame the frames already mask for. The two lnGetUs()
 * calls the burst is bracketed by are separated by sdiTimeUs()'s asm barrier so
 * that the compiler cannot fold them into the wrong order - that, SDI_TRIM_CELLS
 * keeping the burst inside what the timer carries across a mask, and
 * SDI_CELL_MIN_NS..MAX below, is what makes the mask safe. The caller checks the
 * result before using it.
 */
static uint32_t sdiMeasureWriteCellNs(const uint32_t cells)
{
    uint32_t best = 0u;
    for (uint32_t pass = 0; pass < SDI_CAL_PASSES; pass++)
    {
        lnNoInterrupt();
        const uint32_t startUs = sdiTimeUs();
        for (uint32_t i = 0; i < cells; i++)
            sdiSendBit(i & 1u);
        const uint32_t ns = (uint32_t)(((uint64_t)(sdiTimeUs() - startUs) * 1000u) / cells);
        lnInterrupts();
        if (!best || ns < best)
            best = ns;
    }
    return best;
}

/**
 * @brief @p loops scaled so that the cell they are part of comes out at
 *        @p targetNs.
 *
 * Rounded to nearest. The counts are the only knob the model has and the cell
 * responds to them (one count is one iteration, ~42 ns on this part), so moving
 * all of them by one factor is what walks the measured cell onto the target;
 * scaling them together is also what keeps the LOW/HIGH fractions, i.e. the bits.
 * A count of 0 stays 0, which is a half the pad accesses already pay for.
 */
static uint32_t sdiTrim(const uint32_t loops, const uint32_t cellNs, const uint32_t targetNs)
{
    return (loops * targetNs + cellNs / 2u) / cellNs;
}

/* The fewest iterations a data 1's LOW may be made of, *measured* on a CH32V003 with
 * 'mon sdi_wire' (riscv_fault.md §12): the trim below can compress the LOWs to fit a
 * cell, and it must not take this one under 4 - a wire whose 1's LOW was 3 loops
 * attached 1 session of 3, one at 2 loops attached none, and 4-5 attached every
 * session of every boot. The floor costs the cell one iteration (~41 ns, 4%) and is
 * the difference between a session that attaches and one that does not. A count of 0
 * would be worse still: two pad accesses and a loop entry is not a short LOW, it is
 * no LOW at all. */
#define SDI_LOW1_MIN_LOOPS 4u

/**
 * @brief Loop iterations a phase of @p ns needs *on the wire*, with @p fixedNs of
 *        the cell's per-cell constant charged to it.
 *
 * The model sdiPhaseLoops() builds is the CPU's, measured with the pad parked:
 * what it leaves out is what a frame cell spends outside its counted loops - the
 * statics it reloads, the cell it chooses, the outer loop it counts, ~157 ns on a
 * GD32F303 at 96 MHz, a fifth of a cell. That constant is not scalable, so a
 * uniform trim onto the cell cannot place it: it puts the ~157 ns *into* every LOW
 * (that is where the frame spends it, before the edge), which lengthens a data 1's
 * 224 ns LOW to ~294 ns - outside the (T, 2T) window §2.2 gives a 1, i.e. a "1"
 * the target may decode as neither 0 nor 1 - while a data 0's 672 ns LOW has the
 * (4T, 32T) room to absorb it. This is the placement that takes it out again:
 * a write cell's LOW target has the constant subtracted before it is turned into
 * counts, its HIGH target has not, and the two together still come out as one cell
 * of tbitNs (counts * loop + constant), which is the property the trim was for.
 *
 * Rounds to nearest and never returns less than @p minLoops.
 */
static uint32_t sdiWireLoops(const uint32_t ns, const uint32_t fixedNs, const uint32_t minLoops)
{
    const uint32_t loops = (ns > fixedNs) ? (((ns - fixedNs) * 10u) + sdiFrameLoopNsX10 / 2u) / sdiFrameLoopNsX10 : 0u;
    return (loops < minLoops) ? minLoops : loops;
}

/**
 * @brief Turn the waveform's fractions into loop counts, from what the CPU
 *        measures.
 *
 * Two cells of the same shape with different counts: the difference is what the
 * loops cost, and a cell with no counts at all is what the two pad accesses and
 * the two loop entries cost, which gives the per-access cost as well. Both are
 * measured on the pad the frames use, so the result is this part's timing and not
 * a datasheet's.
 */
static void sdiConfigureTiming()
{
    const uint32_t cells = 1000u; /* the empty burst (~0.1 ms); the trim burst is SDI_TRIM_CELLS */
    const uint32_t probeLoops = SDI_PROBE_LOOPS;

    /* The wire: §2.2's numbers, or whatever 'mon sdi_wire' overrode them with.
     * A number above the cell would underflow a HIGH half's target, so the three
     * LOW/sample numbers are clamped to it: the knob is a debug tool and a
     * nonsensical argument should not wrap a phase count into a 4-second one. */
    const SdiWire wire = sdiWireRead();
    const uint32_t tbitNs = sdiWireValue(wire.tbitNs, SDI_TBIT_NS);
    uint32_t low1Ns = sdiWireValue(wire.low1Ns, SDI_LOW1_NS);
    uint32_t low0Ns = sdiWireValue(wire.low0Ns, SDI_LOW0_NS);
    uint32_t sampleNs = sdiWireValue(wire.sampleNs, SDI_RX_SAMPLE_NS);
    if (low1Ns > tbitNs)
        low1Ns = tbitNs;
    if (low0Ns > tbitNs)
        low0Ns = tbitNs;
    if (sampleNs > tbitNs)
        sampleNs = tbitNs;

    /* The two ends of the two-point calibration, both the smallest of
     * SDI_CAL_PASSES bursts and both masked: the empty cell is what the pad
     * accesses and the loop entries cost, the probe cell (SDI_PROBE_LOOPS
     * iterations a half over SDI_PROBE_CELLS cells) what they cost with 200
     * iterations a cell added. The difference divided by those 200 is the loop
     * cost - a number every phase count is derived from, and one no minimum could
     * protect while the burst it came from was long enough to catch a busy window
     * (sdiMeasureProbeCellNs() has the measurement and the defect). */
    const uint32_t emptyNs = sdiMeasureEmptyCellNs(cells);
    const uint32_t probeNs = sdiMeasureProbeCellNs(probeLoops, SDI_PROBE_CELLS);
    sdiEmptyCellNs = emptyNs;
    sdiProbeCellNs = probeNs;
    if (probeNs > emptyNs)
    {
        const uint32_t loopNsX10 = ((probeNs - emptyNs) * 10u) / (2u * probeLoops);
        if (loopNsX10 >= SDI_LOOP_NS_X10_MIN && loopNsX10 <= SDI_LOOP_NS_X10_MAX)
            sdiLoopNsX10 = loopNsX10;
    }
    const uint32_t oneLoopNs = sdiLoopNsX10 / 10u;
    sdiAccessNs = (emptyNs > 2u * oneLoopNs) ? (emptyNs / 2u - oneLoopNs) : 0u;

    /* a write cell, or a header cell: one access in each half */
    sdiLow1Loops = sdiPhaseLoops(low1Ns, 1u);
    sdiHigh1Loops = sdiPhaseLoops(tbitNs - low1Ns, 1u);
    sdiLow0Loops = sdiPhaseLoops(low0Ns, 1u);
    sdiHigh0Loops = sdiPhaseLoops(tbitNs - low0Ns, 1u);
    /* a response cell carries the two shifter turn-arounds and the sample on top
     * of the level it drives: two accesses in its LOW half, three in the half
     * that holds the release, the turn and the sample, and the next cell's
     * turn-around in what is left */
    sdiRxLowLoops = sdiPhaseLoops(low1Ns, SDI_RX_LOW_MIN_LOOPS);
    sdiRxMidLoops = sdiPhaseLoops(sampleNs - low1Ns, SDI_RX_MID_MIN_LOOPS);
    sdiRxTailLoops = sdiPhaseLoops(tbitNs - sampleNs, SDI_RX_TAIL_MIN_LOOPS);

    /* Those counts are a model, and the burst the model was measured in is not a
     * frame: sdiMeasureCellNs() takes its counts as arguments, so the count loads
     * are hoisted out of it, and it runs neither the per-cell register reloads nor
     * the run-time choice of cell that sdiSendWord() pays. On a GD32F303 at
     * 96 MHz the model asks for a 950 ns cell and the frames make 1107 ns: 157 ns
     * that is the cell's rather than the iterations', and that no count in the
     * model can account for.
     *
     * That constant is why the burst below is measured at all. It is a cell in a
     * frame, sdiMeasureWriteCellNs(), so it is the divisor of the trim - and the
     * second reading it leaves (at the counts the trim produced) is the other half
     * of a two-point measurement: between them they give what a loop costs in a
     * frame and what the cell spends outside one, which is what the counts are
     * finally placed with (sdiWireLoops()). Nothing is assumed about the constant:
     * it is a difference between two readings of the same cell, and the loop cost
     * beside it is a difference over a difference of counts. */
    const uint32_t countsUntrimmed = sdiLow1Loops + sdiHigh1Loops;
    uint32_t cellNs = sdiMeasureWriteCellNs(SDI_TRIM_CELLS);
    sdiCellUntrimmedNs = cellNs;
    /* 'mon sdi_wire ... notrim' stops here: the counts the model built are used
     * as they are, which leaves the cell at whatever the per-cell constant makes
     * it. It is the one way to tell the fractions and the cell apart on the wire,
     * and it is also what a boot falls back to whose two readings do not bracket
     * the constant (below). */
    for (uint32_t pass = 0; pass < SDI_TRIM_PASSES && cellNs && !wire.noTrim; pass++)
    {
        /* A cell is ~1 us, and a reading outside SDI_CELL_MIN_NS..SDI_CELL_MAX_NS
         * is not a cell: every count is trimmed by SDI_TBIT_NS / this number, so
         * using it would not degrade the waveform but destroy it (see
         * SDI_CELL_MIN_NS). The model's counts stand instead - they are what the
         * untrimmed cell was built from, and the numbers a part that is not a
         * GD32F303 at 96 MHz would need anyway. */
        if (cellNs < SDI_CELL_MIN_NS || cellNs > SDI_CELL_MAX_NS)
            break;
        const uint32_t offNs = (cellNs > tbitNs) ? (cellNs - tbitNs) : (tbitNs - cellNs);
        if (offNs <= oneLoopNs)
            break; /* one count is one iteration: an integer count cannot do better */
        sdiLow1Loops = sdiTrim(sdiLow1Loops, cellNs, tbitNs);
        sdiHigh1Loops = sdiTrim(sdiHigh1Loops, cellNs, tbitNs);
        sdiLow0Loops = sdiTrim(sdiLow0Loops, cellNs, tbitNs);
        sdiHigh0Loops = sdiTrim(sdiHigh0Loops, cellNs, tbitNs);
        /* The response phases are *not* trimmed: the factor can only move all of
         * their counts together, and what the sample point is is an absolute time
         * behind the falling edge, not a fraction of a cell - the trim is folded
         * into their placement below, against the measured wire. */
        cellNs = sdiMeasureWriteCellNs(SDI_TRIM_CELLS);
    }

    /* The loop above leaves two readings, at two different counts and the same
     * cell shape: sdiCellUntrimmedNs at countsUntrimmed, and cellNs at the counts
     * the trim left. A cell responds linearly in its loops - one iteration is one
     * iteration whatever the shape around it - so the difference between the two
     * readings over the difference between the two counts is what one loop costs
     * *in a frame*, and what is left of a reading once its own loops are paid for
     * is the per-cell constant the model cannot represent. That constant is the
     * number this file has been missing: it is what the trim had to scale the whole
     * waveform to absorb, and scaling the waveform is what moved the response
     * sample point and compressed the LOW fractions with it (riscv_fault.md §11,
     * §12).
     *
     * Both come out of measurements that were taken anyway - no extra burst, and no
     * assumption about where the constant comes from. A pair that does not bracket
     * it (the trim broke out on its first pass, 'mon sdi_wire ... notrim', a cell
     * outside SDI_CELL_MIN_NS..MAX, or a "frame loop" faster than the parked
     * bursts' loop, which no frame path is) leaves them zero and the model's counts
     * stand: exactly the behaviour this file had before they existed. */
    const uint32_t countsTrimmed = sdiLow1Loops + sdiHigh1Loops;
    if (countsTrimmed < countsUntrimmed && cellNs && cellNs < sdiCellUntrimmedNs)
    {
        const uint32_t iterations = countsUntrimmed - countsTrimmed;
        const uint32_t loopX10 = (((sdiCellUntrimmedNs - cellNs) * 10u) + iterations / 2u) / iterations;
        if (loopX10 >= SDI_LOOP_NS_X10_MIN && loopX10 <= SDI_LOOP_NS_X10_MAX)
            sdiFrameLoopNsX10 = loopX10;
    }
    if (sdiFrameLoopNsX10)
    {
        const uint32_t countedNs = (countsUntrimmed * sdiFrameLoopNsX10) / 10u;
        sdiCellFixedNs = (sdiCellUntrimmedNs > countedNs) ? (sdiCellUntrimmedNs - countedNs) : 0u;
    }

    /* The response phases are placed on the *wire* - from the frame loop cost and the
     * per-cell constant measured above - and that is the one thing the trim above must
     * not touch: the sample point is an absolute time behind the falling edge (where the
     * target's answer is), not a fraction of the cell, and riding the trim's factor is
     * what moved it from boot to boot (riscv_fault.md §11.1/§11.2).
     *
     * Measured on a CH32V003 with `mon sdi_wire`: a sample ~8-9 loops behind the edge
     * reads the answer (3 of 3 sessions, 0 faults in 18 section reads), while the same
     * wire sampled at 15 loops (~0.9 us of wire) reads the released line - all ones,
     * "CPBR after unlock = all ones (no slave output)", no session of that boot
     * attaching anything. §2.2's 616 ns absolute is where the good setting lands: the
     * constant is spent between the edge and the sample, so the counted part of the
     * interval is (sample - fixed) over the loop cost, and the mid phase keeps its floor
     * when that would make it shorter than the target needs to drive its bit. The tail
     * then takes what is left of the cell, so the read clock runs at the write clock's
     * period whatever the floor took out of the mid phase.
     *
     * The write cells keep the trim's counts and not this placement: they were measured
     * onto a 930 ns cell that attaches in every session of every boot, and §2.2's
     * fractions at this CPU's cost are 21 loops, i.e. a ~1180 ns cell - 1.3x slower,
     * which attached in 2 sessions of 3 when it was tried (riscv_fault.md §12). A cell
     * that attaches is worth more than a LOW that is 25% longer than §2.2 asks for,
     * especially with the read path now re-issuing what a mis-decoded frame costs. */
    if (sdiFrameLoopNsX10 && !wire.noTrim)
    {
        sdiRxLowLoops = sdiWireLoops(low1Ns, 0u, SDI_RX_LOW_MIN_LOOPS);
        const uint32_t sampleLoops =
            sdiWireLoops(sampleNs, sdiCellFixedNs, sdiRxLowLoops + SDI_RX_MID_MIN_LOOPS);
        sdiRxMidLoops = sampleLoops - sdiRxLowLoops;
        const uint32_t trimmedCellLoops = sdiLow1Loops + sdiHigh1Loops;
        sdiRxTailLoops = (trimmedCellLoops > sdiRxLowLoops + sdiRxMidLoops)
            ? (trimmedCellLoops - sdiRxLowLoops - sdiRxMidLoops)
            : SDI_RX_TAIL_MIN_LOOPS;
    }

    /* And the floors again: the loop above only knows the factor, so it can land
     * a response phase under the floor sdiPhaseLoops() gave it - a boot that
     * leaves the loop with sdiRxMidLoops == 1 samples the response ~380 ns early
     * and cannot attach a CH32V003 at all. */
    if (sdiLow1Loops < SDI_LOW1_MIN_LOOPS)
        sdiLow1Loops = SDI_LOW1_MIN_LOOPS;
    if (sdiRxLowLoops < SDI_RX_LOW_MIN_LOOPS)
        sdiRxLowLoops = SDI_RX_LOW_MIN_LOOPS;
    if (sdiRxMidLoops < SDI_RX_MID_MIN_LOOPS)
        sdiRxMidLoops = SDI_RX_MID_MIN_LOOPS;
    if (sdiRxTailLoops < SDI_RX_TAIL_MIN_LOOPS)
        sdiRxTailLoops = SDI_RX_TAIL_MIN_LOOPS;

    /* What this calibration was built from: a change to it is what makes the next
     * mode entry calibrate again (sdiWireChanged()). */
    sdiWireUsed = wire;
}

/**
 * @brief One console line: the transport, what it measured, and what a cell
 *        comes out as.
 *
 * Under 127 characters, and the measurements first: Logger() formats into a
 * 128 byte static buffer (lnDebug.cpp, vsnprintf(buffer, 127, ...)), so a longer
 * line is cut at the buffer and loses its tail *and* its newline - which is what
 * hid the untrimmed cell and the wire numbers this line used to end with, and why
 * the mode entry lines run into the one after them in a capture. So: the loop cost
 * the parked bursts measured with the two bursts it came out of (empty, probe),
 * then the wire's own loop cost and the per-cell constant the counts are placed
 * with (sdiWireLoops()), then the cell the counts make against the cell they made
 * before the trim - which is what says whether the measurement ran in a quiet
 * window, and how far the model was off - and then the counts, i.e. what the wire
 * will do. `mon sdi_wire` prints what the wire was asked for.
 *
 * The cell is measured (SDI_TRIM_CELLS cells, pad parked off the wire, see
 * sdi_pinmode_enter()), so the last number is the CPU's cost and the burst carries
 * nothing: mode entry is before sdi_dm_start() resets the target, and nothing is
 * listening for a cell before that either.
 *
 * Single Logger call on purpose - the format buffer is shared.
 */
static void sdiLogMode()
{
    const uint32_t cellNs = sdiMeasureWriteCellNs(SDI_TRIM_CELLS);

    Logger("SDI : pin %u : loop %u.%u ns (empty %u probe %u), wire %u.%u+%u, cell %u/%u, LOW1 %u+%u, LOW0 %u+%u, "
           "rx %u/%u/%u\n",
           (unsigned)SDI_DATA_PIN, (unsigned)(sdiLoopNsX10 / 10u), (unsigned)(sdiLoopNsX10 % 10u),
           (unsigned)sdiEmptyCellNs, (unsigned)sdiProbeCellNs, (unsigned)(sdiFrameLoopNsX10 / 10u),
           (unsigned)(sdiFrameLoopNsX10 % 10u), (unsigned)sdiCellFixedNs, (unsigned)cellNs,
           (unsigned)sdiCellUntrimmedNs, (unsigned)sdiLow1Loops, (unsigned)sdiHigh1Loops,
           (unsigned)sdiLow0Loops, (unsigned)sdiHigh0Loops, (unsigned)sdiRxLowLoops, (unsigned)sdiRxMidLoops,
           (unsigned)sdiRxTailLoops);
}

// ---------------------------------------------------------------------------
// Frame content. The same shape the timer/DMA tap builds into its duties[]
// tables - one writer per cell, contiguous cell by cell - except that a cell is
// made straight from its bit: the loop counts are per bit value, so there is
// nothing to fill in.
// ---------------------------------------------------------------------------
/** @brief Make the nine header cells of a frame, start bit first. */
static void sdiSendHeader(const uint8_t adr, const uint8_t mode)
{
    const uint32_t header = make_sdi_header(adr, mode);
    for (int i = 0; i < SDI_HEADER_BITS; i++)
        sdiSendBit((header >> (SDI_HEADER_BITS - 1 - i)) & 1U);
}

/** @brief Make the 32 payload cells of a write, MSB first (sdi.pio order). */
static void sdiSendWord(const uint32_t data)
{
    for (int i = 0; i < SDI_WORD_BITS; i++)
        sdiSendBit((data >> (SDI_WORD_BITS - 1 - i)) & 1U);
}

// ---------------------------------------------------------------------------
// Frame level: the same wire format as the timer/DMA tap (and
// bmp_sdiTap_rp2040_riscv_extra.cpp) - a header of nine cells and 32 cells of payload or
// response, contiguous, then the §2.2 idle.
// ---------------------------------------------------------------------------
/**
 * @brief Sample the 32 response cells that follow a read header, MSB first.
 *
 * Per cell (sdi.pio's rx_loop, with the CPU doing what the DIR DMA channels did
 * there): sink the wire for one read clock (LOW1 - a read clock carries no data
 * of its own, §2.2), release it and hand the shifter to the target, sample the
 * pad at the cell's sample point, then wait out the rest of the cell.
 *
 * Order at the release is release-then-turn: the pad stops driving before the
 * shifter starts passing the target's level, so the two never drive the wire in
 * opposite directions however many cycles apart the two stores land. The turn
 * back to the probe is at the head of the next cell, i.e. before the next LOW,
 * which is where the phase counts expect it (sdiRxLowLoops has room for it).
 *
 * The three phases are counted from the same instant, so the sample sits the same
 * distance behind the falling edge as in the RP2040 timing (616 - 224 ns) and a
 * phase that comes out a few iterations long moves the sample with it: §2.2's
 * response window is wide (the target holds its bit until the next falling edge),
 * so a later sample still reads the bit it was meant to read.
 */
static uint32_t sdiSampleWord()
{
    uint32_t word = 0;

    for (int i = 0; i < SDI_WORD_BITS; i++)
    {
        sdiDirProbe(); /* the probe's LOW has to reach the wire */
        sdiPadLow();
        sdiLoop(sdiRxLowLoops);
        sdiPadRelease();
        sdiDirTarget(); /* the target drives the wire from here */
        sdiLoop(sdiRxMidLoops);
        word = (word << 1) | (sdiReadPad() ? 1U : 0U);
        sdiLoop(sdiRxTailLoops);
    }
    return word;
}

/**
 * @brief Send one DM write: the nine header cells and the 32 payload cells as
 *        *one* contiguous packet, then idle high.
 *
 * One packet, not two frames: §2.2's stop is a HIGH of 10 T or more, so a gap
 * between the header and the payload ends the packet after the header and the
 * payload is then parsed as a packet of its own.
 *
 * Interrupts stay masked for the frame: an ISR inside a cell would stretch the
 * LOW or the HIGH by its own runtime, and a stretched LOW can be read as the
 * other bit. Nothing in the loop blocks, so - unlike the DMA path - the mask
 * cannot park the probe: the CPU's own instructions are the only thing it waits
 * for.
 */
static void sdiWriteFrame(const uint8_t adr, const uint32_t data)
{
    lnNoInterrupt();
    sdiPadDriven(); /* the probe drives both levels of a write */
    sdiDirProbe();
    sdiSendHeader(adr, SDI_WRITE_FLAG);
    sdiSendWord(data);
    sdiPadRelease(); /* the stop cell: HIGH, §2.2 needs >= 10 T */
    lnInterrupts();

    lnDelayUs(END_OF_WRITE_DELAY);
    /* The same line the timer/DMA tap prints, so the two transports' consoles
     * read the same - off unless the transport itself is being debugged
     * (`mon memlog on`), see bmp_mem_log_enabled above. */
    if (bmp_mem_log_enabled)
        Logger("SDI : write 0x%02x = 0x%x : one packet, %d cells\n", (unsigned)adr, (unsigned)data,
               SDI_HEADER_BITS + SDI_WORD_BITS);
}

/**
 * @brief Send one DM read: the nine header cells, then the 32 response cells
 *        clocked out by the CPU while it samples them.
 *
 * The header is write-like - the probe drives both levels - so the pad stays
 * push-pull until the responses start, where only the LOW may be driven and the
 * HIGH has to be a release. The pad mode changes between the header and the first
 * response cell, i.e. once per frame and not inside a cell: the target starts its
 * response at the fall of that first clock either way, so the switch cannot shift
 * a bit.
 */
static uint32_t sdiReadFrame(const uint8_t adr)
{
    uint32_t word;

    lnNoInterrupt();
    sdiPadDriven();
    sdiDirProbe();
    sdiSendHeader(adr, SDI_READ_FLAG);

    sdiPadOpenDrain(); /* the HIGH becomes a release: the target can drive */
    sdiPadRelease();
    word = sdiSampleWord();
    sdiPadRelease(); /* the stop cell: HIGH from here on */
    lnInterrupts();

    sdiDirProbe();
    lnDelayUs(INTER_WORD_DELAY);
    /* The mirror of the write line in sdiWriteFrame(): with it a `mon memlog on`
     * capture is a complete DMI trace - both directions - instead of the write half
     * of one, which matters when the question is "what did DATA0 answer". Off
     * unless the transport itself is being debugged, for the same reason it is
     * there (see bmp_mem_log_enabled). */
    if (bmp_mem_log_enabled)
        Logger("SDI : read  0x%02x = 0x%x : one packet, %d cells\n", (unsigned)adr, (unsigned)word,
               SDI_HEADER_BITS + SDI_WORD_BITS);
    return word;
}

// ---------------------------------------------------------------------------
// Pin mode side: ownership of PB8/PC3 for the duration of an SDI session. Called
// by bmp_gpio_pinmode() in bmp_tap_ln.cpp, the LN platform file that knows every
// mode (mirroring bmp_tap_rp2040.cpp on the RP2040 side).
// ---------------------------------------------------------------------------
/**
 * @brief Enter SDI mode: park PB8 released and calibrate the timing once.
 *
 * PB8 leaves the tap in open drain and released, with the shifter pointing at the
 * probe, which is the state every frame starts and ends in. There is no cell
 * engine to build here: that is the point of this file.
 *
 * The calibration and the trim it feeds are the only cells a session makes outside
 * a frame, so they are made with the pad held off the wire: they time the CPU, not
 * the wire - sdiSendCell()'s stores, to the same BOP register at the same address
 * as a frame's - so a pin mode whose output driver is off leaves the shifter at its
 * idle high and the ~12 ms of cells they make reach nobody. Both modes have the ODR
 * bit high (open drain released before, pull-up after), so the pin does not move
 * either way. (SWCLK/PB9 would measure the same store too - lnFastIO::on() writes
 * the same port register with a different bit - but this keeps the measurement on
 * the pin, and on the code path, the frames use.)
 */
void sdi_pinmode_enter()
{
    if (!rSWDIO)
        return;
    sdiPark(); /* also what the measurement has to start from: same state every session */
    /* Once per session - and again whenever 'mon sdi_wire' moved the wire under it,
     * since the burst has to be measured against the waveform the frames will make.
     * This is the one place the pad is parked off the wire for that measurement. */
    if (!sdiTimingDone || sdiWireChanged())
    {
        lnPinMode(SDI_DATA_PIN, lnINPUT_PULLUP); /* driver off: the measurement is the CPU's */
        sdiConfigureTiming();
        sdiLogMode();
        sdiPark();
        sdiTimingDone = true;
    }
    sdiMode = true;
}

/**
 * @brief Leave SDI mode: give the pad back released.
 *
 * PB8/PC3 are left in the state the SWD/RVSWD taps start from (released open
 * drain, direction at the probe, PB8 forced high by SwdPin::on()), so the next
 * swdptap_init()/rvswd_scan() takes over without a fight on the wire.
 */
void sdi_pinmode_leave()
{
    if (!sdiMode)
        return;
    sdiMode = false;
    if (rSWDIO)
    {
        sdiPadRelease();
        sdiDirProbe();
        sdiPadOpenDrain();
    }
    Logger("SDI : leaving SDI mode\n");
}

// EOF



