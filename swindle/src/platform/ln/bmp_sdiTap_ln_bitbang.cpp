/**
 * @file bmp_sdiTap_ln_bitbang.cpp
 * @brief WCH SDI tap for the LN platform: CPU-timed (bit-bang) transport.
 *
 * The sibling file bmp_sdiTap_ln.cpp drives the wire with a timer channel plus
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

// The blackmagic RISC-V target framework (riscv_debug.h) is needed for the
// stage-2 attach below, exactly as in the timer/DMA tap: we hand
// riscv_dmi_init() an SDI-backed riscv_dmi_s, mirroring what
// rvswd_template.h::rvswd_scan() does over RVSWD.
extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
extern "C" void target_list_free(void);

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
// Bit cell (§2.2, T = 896 ns for the 1x time base the RP2040 tap was brought up
// with, sdi.pio): one cell per bit, a 1 is a LOW of 1/4 cell (224 ns) and a 0 a
// LOW of 3/4 cell (672 ns), the wire high for the rest of the cell, and the
// target's response is sampled 11/16 of a cell (616 ns) after the boundary.
//
// The same fractions the timer/DMA tap uses, in nanoseconds instead of timer
// ticks: this transport's clock is a counted NOP loop, so the numbers below are
// the waveform and sdiConfigureTiming() turns them into loop counts, measured on
// the part rather than assumed.
// ---------------------------------------------------------------------------
#define SDI_TBIT_NS 896u      /* one cell */
#define SDI_LOW1_NS 224u      /* 1/4 cell: the LOW that carries a 1 */
#define SDI_LOW0_NS 672u      /* 3/4 cell: the LOW that carries a 0 */
#define SDI_RX_SAMPLE_NS 616u /* 11/16 cell: where a response bit is sampled */

/* Fallback loop cost, in tenths of a nanosecond: 12.5 ns per iteration is the
 * figure bmp_set_frequency_c() calibrates SWD from on a GD32F303 at 96 MHz. It
 * only stands in until the measurement has run. */
#define SDI_LOOP_NS_X10_DEFAULT 125u

/* What the measurement found, and the loop counts derived from it. The counts
 * stay zero until sdiConfigureTiming() runs, which every frame is behind: frames
 * only start once bmp_gpio_pinmode(BMP_PINMODE_SDI) has called
 * sdi_pinmode_enter() (see sdi_dm_start()). */
static uint32_t sdiLoopNsX10 = SDI_LOOP_NS_X10_DEFAULT; /* one loop iteration */
static uint32_t sdiAccessNs = 0;                        /* one pad access */
static uint32_t sdiLow1Loops = 0, sdiHigh1Loops = 0;    /* a cell whose bit is 1 */
static uint32_t sdiLow0Loops = 0, sdiHigh0Loops = 0;    /* a cell whose bit is 0 */
static uint32_t sdiRxLowLoops = 0, sdiRxMidLoops = 0, sdiRxTailLoops = 0;

/* SDI owns PB8/PC3 for the whole session; the timing is calibrated once. */
static bool sdiMode = false;
static bool sdiTimingDone = false;

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
 */
static uint32_t sdiMeasureCellNs(const uint32_t lowLoops, const uint32_t highLoops, const uint32_t cells)
{
    const uint32_t startUs = lnGetUs();
    for (uint32_t i = 0; i < cells; i++)
        sdiSendCell(lowLoops, highLoops);
    return ((lnGetUs() - startUs) * 1000u) / cells;
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
    const uint32_t cells = 1000u;
    const uint32_t probeLoops = 100u; /* long enough that the loops dominate the measurement */

    const uint32_t emptyNs = sdiMeasureCellNs(0u, 0u, cells);
    const uint32_t probeNs = sdiMeasureCellNs(probeLoops, probeLoops, cells);
    if (probeNs > emptyNs)
    {
        const uint32_t loopNsX10 = ((probeNs - emptyNs) * 10u) / (2u * probeLoops);
        if (loopNsX10)
            sdiLoopNsX10 = loopNsX10;
    }
    const uint32_t oneLoopNs = sdiLoopNsX10 / 10u;
    sdiAccessNs = (emptyNs > 2u * oneLoopNs) ? (emptyNs / 2u - oneLoopNs) : 0u;

    /* a write cell, or a header cell: one access in each half */
    sdiLow1Loops = sdiPhaseLoops(SDI_LOW1_NS, 1u);
    sdiHigh1Loops = sdiPhaseLoops(SDI_TBIT_NS - SDI_LOW1_NS, 1u);
    sdiLow0Loops = sdiPhaseLoops(SDI_LOW0_NS, 1u);
    sdiHigh0Loops = sdiPhaseLoops(SDI_TBIT_NS - SDI_LOW0_NS, 1u);
    /* a response cell carries the two shifter turn-arounds and the sample on top
     * of the level it drives: two accesses in its LOW half, three in the half
     * that holds the release, the turn and the sample, and the next cell's
     * turn-around in what is left */
    sdiRxLowLoops = sdiPhaseLoops(SDI_LOW1_NS, 2u);
    sdiRxMidLoops = sdiPhaseLoops(SDI_RX_SAMPLE_NS - SDI_LOW1_NS, 3u);
    sdiRxTailLoops = sdiPhaseLoops(SDI_TBIT_NS - SDI_RX_SAMPLE_NS, 1u);
}

/**
 * @brief One console line: the transport, what it measured, and what a cell
 *        comes out as.
 *
 * The last number is the interesting one: sdiMeasureCellNs() on the counts that
 * were just derived, i.e. the cell the target will actually see. It clocks 1000
 * cells onto the wire, which happens at mode entry, before sdi_dm_start() resets
 * the target: nothing is listening for them yet.
 *
 * Single Logger call on purpose - Logger() formats into one shared static buffer
 * that the output path drains asynchronously, so two back-to-back calls can drop
 * the first line.
 */
static void sdiLogMode()
{
    Logger("SDI : bit-bang transport on pin %u : loop %u.%u ns, access %u ns, LOW1 %u+%u loops, LOW0 %u+%u, "
           "rx %u/%u/%u, one cell measures %u ns\n",
           (unsigned)SDI_DATA_PIN, (unsigned)(sdiLoopNsX10 / 10u), (unsigned)(sdiLoopNsX10 % 10u),
           (unsigned)sdiAccessNs, (unsigned)sdiLow1Loops, (unsigned)sdiHigh1Loops, (unsigned)sdiLow0Loops,
           (unsigned)sdiHigh0Loops, (unsigned)sdiRxLowLoops, (unsigned)sdiRxMidLoops,
           (unsigned)sdiRxTailLoops, (unsigned)sdiMeasureCellNs(sdiLow1Loops, sdiHigh1Loops, 1000u));
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
// bmp_sdiTap_rp2040.cpp) - a header of nine cells and 32 cells of payload or
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
     * read the same. */
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
 * probe, which is the state every frame starts and ends in - and the state the
 * calibration (which drives the pad) has to measure from. There is no cell engine
 * to build here: that is the point of this file.
 */
void sdi_pinmode_enter()
{
    if (!rSWDIO)
        return;
    if (!sdiMode)
    {
        sdiPadRelease();
        sdiDirProbe();
        sdiPadOpenDrain();
    }
    if (!sdiTimingDone)
    {
        sdiConfigureTiming(); /* measures the pad too: park it first */
        sdiLogMode();
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



