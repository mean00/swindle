/*
 */
/**
 * @file bmp_sdiTap_ln.cpp
 * @brief WCH SDI (single wire debug interface) tap for the LN platform.
 *
 * Same abstract interface as bmp_sdiTap_rp2040.cpp (sdi_dm_start/write/read, the
 * bmp_sdi_dm_*_c entry points, sdi_scan, the ch32_sdi_dmi_* leaves and every
 * protocol constant), but no PIO:
 *
 *  - every cell of every frame comes from one fixed period PWM carrier: one
 *    compare event per cell, the duty (CHCV = the cell's LOW time) written by the
 *    CPU one cell ahead, so the *edge* - and the edge is the whole cell length -
 *    is always the timer's and never depends on what the CPU is doing,
 *  - no DMA: a write frame's 41 duties are stored to CHCV one per cell boundary,
 *    a read frame needs no store at all after its header (every response cell has
 *    the same LOW1 duty), so the CPU only waits for the boundary (the update
 *    flag), turns the shifter around once per response cell and reads the pad at
 *    the sample tick: ~6 bus accesses per cell, all of it inside lnNoInterrupt()/
 *    lnInterrupts() (an ISR inside a cell would push a sample past the boundary).
 *
 * A CPU made cell does not fit in a cell: 86 ticks at 96 MHz against four pad
 * stores plus the boundary poll, ~10 bus accesses at ~9 cycles each. Measured, a
 * hand made read cell came out at 1.5 us and the target was handed a clock 1.7x
 * too slow - a *wrong* clock, not a slow one - and the protocol's own windows
 * ((T,2T) LOW, (T,8T) HIGH, sdi.txt §2) leave no room to stretch the cell and buy
 * the CPU the time either. See §13 of the plan. One CHCV store per cell, against
 * the same budget, is 9 cycles.
 *
 * The direction pin is *not* part of a response cell on this platform: the level
 * shifter's direction (PC3) is left undriven for the whole SDI session and the board's
 * own default decides it - see SDI_LN_DRIVE_DIR_PIN further down for what that means
 * and how to bring the stores back. The RP2040 tap writes that pin once, in
 * setupSDI() (bmp_tap_rp2040.cpp), and turns the *wire* around per bit with the PIO's
 * own pin direction. A timer channel cannot do that - PB8's alternate function has no
 * high-Z release - which is why the LN tap used to turn the shifter per cell instead
 * (two stores, at ticks the loop was waiting for anyway, replacing the DMA that used
 * to fire them at the release) and turns it not at all now.
 *
 * The bit cell, the frame layout and every protocol constant are shared with the
 * RP2040 tap (sdi.pio, sdi.txt); only the way the waveform is produced differs.
 * bmp_sdiTap_ln_bitbang.cpp is the older CPU paced variant, kept for bring-up:
 * CMakeLists.txt builds exactly one of the two.
 *
 * Pins: the SDI wire is the SWDIO line, _mapping[TSWDIO_PIN] (PB8), whose
 * pinMappings entry maps it to timer 3 channel 2, i.e. TIMER4_CH3;
 * _mapping[TDIRECTION_PIN] (PC3) is the level shifter direction, owned by
 * SwdDirectionPin *rSWDIO (bmp_tap_ln.cpp) and not touched by SDI. NRST is driven
 * through the shared SwdReset controller so the build time polarity option is
 * honoured.
 */

// -------------------------------------------------- //
#include "lnGPIO.h"
#include "lnTimer.h"
#include "lnTimer_priv.h"
#include "lnPinMapping.h"
#include "stdint.h"
#include "esprit.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_tap_ln.h"
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
extern "C" void target_list_free(void);

/* SDI data pin: the same physical wire the SWD/RVSWD taps use, i.e. the SWDIO
 * line _mapping[TSWDIO_PIN] (PB8 on both packages).
 *
 * PB8 changes role per frame, never inside one: the timer's channel drives it
 * (alternate function - push-pull for a write frame, open drain for a read one)
 * and the GPIO takes it back released between frames. _fast is the pad's BOP
 * set/clear word, one store, and it is the only one of the two registers of the
 * tap's pin group that this platform writes (see SDI_LN_DRIVE_DIR_PIN). */
#define SDI_DATA_PIN _mapping[TSWDIO_PIN]

/* The tap's second pin is the level shifter's direction (PC3, SwdDirectionPin's
 * _fastdir). The RP2040 tap writes it once, in setupSDI(), and turns the wire with
 * the PIO's pin direction instead; this platform does not drive it at all - not for
 * the session, not inside a response cell. PC3 is therefore left exactly as SDI found
 * it and the board's own default decides the direction; the SWD and RVSWD taps set it
 * themselves in their constructor, so whichever tap runs next is unaffected either
 * way. Set to 1 to bring the stores back: sdiDirProbe(), sdiDirTarget() and the two
 * parks in sdi_pinmode_enter()/sdi_pinmode_leave(). */
#define SDI_LN_DRIVE_DIR_PIN 0

// ---------------------------------------------------------------------------
// Platform transport hooks consumed by the shared protocol layer.
//
// The SDI protocol itself (DM/configuration registers, §2.4 shadow/commit, frame
// format, CPBR diagnostics, DMI glue and the RISC-V attach) lives in
// ../../template/sdi_template.h, shared with the other platforms. This file owns
// the wire: the timer/DMA write frame and the CPU-timed read frame below.
//
// The hooks are macros, not functions: the template only ever calls them once
// per frame (never inside a bit cell), so the indirection costs nothing and the
// frame-level code expands to exactly the same source on every platform.
// ---------------------------------------------------------------------------
static void sdiWriteFrame(const uint8_t adr, const uint32_t data);
static uint32_t sdiReadFrame(const uint8_t adr);

#define sdiFrameWrite(adr, val) sdiWriteFrame((adr), (val))
#define sdiFrameRead(adr) sdiReadFrame(adr)
/* §2.4(3): hold the wire low for @p ms, then let it float high again. The pulse
 * has to come from the pad itself: ODR 0 sinks it through the open drain, and
 * releasing ODR lets the level shifter's pull-up float the wire high. The shifter's
 * direction is whatever the board gives it, SDI never driving PC3. */
#define sdiWireHoldLow(ms)                                 \
    do                                                     \
    {                                                      \
        rSWDIO->_fast.off();   /* Drive LOW */             \
        lnDelayMs(ms);                                     \
        rSWDIO->_fast.on();    /* Release to float HIGH */ \
    } while (0)

/* The shared protocol layer (sdi_dm_start/write/read, bmp_sdi_dm_*_c,
 * ch32_sdi_dmi_read/write, sdi_scan). Requires: bmp_pinmode.h, pReset,
 * rSWDIO/bmp_tap_ln.h, Logger/lnDelayMs/lnDelayUs, <cstring> and the blackmagic
 * RISC-V target framework, all included above. */
#include "sdi_template.h"



/* Bit cell (§2.2, T = 896 ns for the 1x time base the RP2040 tap was measured
 * against). One cell = one LOW pulse whose length selects the bit, then HIGH
 * until the next cell. The response is sampled at 616 ns (the point sdi.pio
 * settles on, ~116 ns above the 500 ns the text guarantees for the shortest
 * data-0 LOW) and the data stays valid until the end of the cell, so the CPU has
 * the rest of the cell to read the pin and re-arm for the next boundary.
 *
 * Every one of those times is an exact fraction of the cell (1/4, 11/16, 3/4),
 * so they are derived from the cell length in ticks instead of asking the timer
 * clock again: one clock measurement per session, and the split stays exact
 * whatever the clock does. */
#define SDI_TBIT_NS 896u
#define SDI_LOW1_NUM 1u /* LOW1 = 1/4 cell, 224 ns */
#define SDI_LOW1_DEN 4u
#define SDI_RX_SAMPLE_NUM 11u /* sample = 11/16 cell, 616 ns */
#define SDI_RX_SAMPLE_DEN 16u

/* Cell frequency handed to lnDmaTimer, which measures the timer clock (and knows
 * about the ARM APB1 x2 workaround) and turns this into CAR: one compare event
 * per cell, i.e. 86 ticks at 96 MHz and 129 at 144 MHz. */
#define SDI_CELL_HZ (1000000000u / SDI_TBIT_NS)

/* The duty of a cell is one CHCV store, made at the boundary that starts the cell
 * before it: 9 cycles against a cell of 86 ticks, and cheap enough to do *after*
 * the sample in a response cell. The old CPU made LOW needed extra ticks instead
 * (the pad was pulled low a couple of cycles after the boundary, so the pulse came
 * out shorter than the 224 ns minimum): with the timer making the edge there is
 * nothing to compensate for - CHCV is the LOW length. */

/**
 * @brief ticks * num / den, rounded up.
 *
 * Used for the LOW times and the sample point: those have documented minima
 * (§2.2), so they round up rather than to nearest.
 */
static uint32_t sdiTicksUp(uint32_t ticks, uint32_t num, uint32_t den)
{
    return (ticks * num + den - 1) / den;
}

}
// ---------------------------------------------------------------------------
// Cell engine.
//
// The RP2040 drives the whole frame from one PIO state machine; here one PWM
// carrier drives it and the two halves of a frame differ only in who writes the
// duties and in what the pad does with its HIGH:
//
//   write frame : the CPU stores the 41 duties, one per cell boundary, and the
//                 pad is push-pull - the probe drives both levels,
//   read frame  : the same carrier with the pad in open drain, so each HIGH is a
//                 release the target answers on. Only the nine header cells need
//                 a duty: every response cell is LOW1, so the CPU has nothing to
//                 store there and only waits for the boundary and the sample.
//
// PWM mode 2 (esprit's lnTimerModePwm1, OCxM = 111) is what makes CHCV mean "the
// LOW time of the cell" and makes a duty of 0 mean "never low", so the stop cell
// needs no special case. OCxPE (LN_TIME_CHCTL0_SEN) is enabled so the duty stored
// during cell k becomes active at the start of cell k+1.
// ---------------------------------------------------------------------------
extern LN_Timers_Registers *abTimers[5];
#define aTimer(x) abTimers[x]

/* The pin -> (timer, channel) lookup is lnDmaTimer's: its constructor walks
 * pinMappings with the key 10 * timer + channel, and pwmSetup() is where the
 * timer clock is measured and turned into CAR. PB8 is timer 3 / channel 2, i.e.
 * TIMER4_CH3, whose timerMappings entry {32, 0, 4} is the DMA request line that
 * used to carry the duties and is not used here any more. */

/**
 * @brief Fixed period SDI cell engine: one compare event per cell, the duty fed
 *        by the CPU one cell ahead of it.
 *
 * lnDmaTimer is the base for what it does in its constructor and in pwmSetup():
 * the pin's timer/channel, and the cell period, i.e. one measurement of the timer
 * clock with the ARM APB1 x2 workaround in it. Its lnDMA member is not used here
 * any more - no transfer is ever armed - but it stays allocated: it is the channel
 * PB8's timer channel has always taken (TIMER4_CH3 -> engine 0 channel 4) and
 * nothing else asks for it.
 *
 * What is SDI specific above the base: a frame is *one shot* - a preloaded first
 * duty, a stop cell, the counter stopped at the end - where lnDmaTimer::start()
 * is circular, interrupt driven and starts on rollover/2, and the duties reach
 * CHCV from the CPU instead of from a transfer.
 */
class SdiTimer : public lnDmaTimer
{
  public:
    /** 16 bit duties: one cell per compare event, CHCV = that cell's LOW time. */
    SdiTimer(lnPin pin) : lnDmaTimer(16, pin)
    {
        _t = aTimer(_timer);
        _period = _low1 = _low0 = _sample = 0;
    }
    /** Measure the timer clock, set the cell period, prepare the channel. */
    void configure();
    /** Start the carrier, feed it @p cells duties; @return the late boundaries. */
    int runCells(const uint16_t *duties, int cells);
    /** Preload the next cell's duty: it becomes active at the next boundary. */
    void preload(uint16_t duty);
    /** Wait for the next cell boundary, and never drop a cell. */
    bool waitCellBoundary();
    /** Wait for the current cell's sample point: where the pad is read. */
    void waitSampleTick()
    {
        waitTick(_sample);
    }
    /** Wait for the carrier's release in the current cell (its LOW is over). */
    void waitRelease()
    {
        waitTick(_low1);
    }
    /** Wait for the stop cell: every real cell is over and the wire is high. */
    void waitStopCell();
    /** Stop the carrier, leaving the channel output where it is (high). */
    void stopCarrier();
    //-- derived tick counts
    uint32_t low1() const
    {
        return _low1;
    }
    uint32_t low0() const
    {
        return _low0;
    }
    uint32_t period() const
    {
        return _period;
    }
    uint32_t sampleTick() const
    {
        return _sample;
    }
    void logTicks();

  protected:
    LN_Timers_Registers *_t;
    uint32_t _period, _low1, _low0, _sample;
    void waitTick(uint32_t tick);
    uint32_t readChannelCtl(int channel) const;
    void writeChannelCtl(int channel, uint32_t value);
};

/** @brief The 8-bit channel control word of one of the timer's channels. */
uint32_t SdiTimer::readChannelCtl(int channel) const
{
    return (_t->CHCTLs[channel >> 1] >> (8 * (channel & 1))) & 0xff;
}

void SdiTimer::writeChannelCtl(int channel, uint32_t value)
{
    const int shift = 8 * (channel & 1);
    uint32_t reg = _t->CHCTLs[channel >> 1];
    reg &= ~(0xffU << shift);
    reg |= (value & 0xffU) << shift;
    _t->CHCTLs[channel >> 1] = reg;
}

/** @brief Report the derived cell timings, so a clock mismatch is visible. */
void SdiTimer::logTicks()
{
    Logger("SDI : timer %d channel %d : cell %u ticks, LOW1 %u, LOW0 %u, sample %u\n", (int)_timer, (int)_channel,
           (unsigned)_period, (unsigned)_low1, (unsigned)_low0, (unsigned)_sample);
}

/**
 * @brief Configure the carrier: one compare event per cell, PWM mode 2 with the
 *        preload on, counter stopped.
 *
 * The cell period comes from lnDmaTimer::pwmSetup(), i.e. from one measurement of
 * the timer clock (APB1 x2 workaround included), and is read back as rollover().
 * LOW1, the sample point and LOW0 are then exact fractions of that period rather
 * than a second measurement, so they cannot drift against the cell: 86 ticks give
 * 22 / 64 / 60 at 96 MHz, 129 give 33 / 96 / 89 at 144 MHz.
 *
 * CHCV is left at 0, i.e. the channel output is HIGH: that is the state the pad
 * is handed over to the timer in, so the hand-over produces no edge.
 */
void SdiTimer::configure()
{
    pwmSetup((int)SDI_CELL_HZ);
    setMode(lnTimerModePwm1); // pwmSetup() leaves the channel in PWM mode 1
    _period = (uint32_t)rollover();

    _low1 = sdiTicksUp(_period, SDI_LOW1_NUM, SDI_LOW1_DEN);
    if (_low1 < 1)
        _low1 = 1;
    if (_low1 >= _period)
        _low1 = _period - 1;
    _low0 = _period - _low1; // keep the split exact: LOW1 + LOW0 == period, so LOW0 comes out
                             // at 672 ns minus the rounding of LOW1 (667 ns at 96/144 MHz)
    _sample = sdiTicksUp(_period, SDI_RX_SAMPLE_NUM, SDI_RX_SAMPLE_DEN);
    if (_sample < _low1 + 1)
        _sample = _low1 + 1; // the sample must follow the clock LOW
    if (_sample >= _period)
        _sample = _period - 1; // and it must stay inside the cell

    _t->CTL0 &= ~LN_TIMER_CTL0_CEN;
    writeChannelCtl(_channel, readChannelCtl(_channel) | LN_TIME_CHCTL0_SEN); // OCxPE: shadowed duty
    _t->CHCTL2 &= ~(LN_TIMER_CHTL2_CHxP(_channel)); // active high
    _t->CHCTL2 |= LN_TIMER_CHTL2_CHxEN(_channel);   // stays enabled for the session
    _t->CHCVs[_channel] = 0;                        // duty 0: never low, i.e. the wire idles high
    _t->INTF = 0;
    _t->SWEV |= LN_TIMER_SWEVG_UPG; // commit CAR/CHCV to the active registers
}

/** @brief Preload the next cell's duty: it becomes active at the next boundary. */
void SdiTimer::preload(uint16_t duty)
{
    _t->CHCVs[_channel] = duty;
}

/**
 * @brief Start the carrier and feed it @p cells cells out of @p duties.
 *
 * duties[0] is committed by an update event *before* the counter starts, so cell
 * 0's LOW begins at the frame's t0 - the target may already be sampling the wire
 * when the frame starts, so cell 0 cannot be the one that waits for a boundary -
 * and duties[k] (k = 1..cells) is stored at the boundary that starts cell k-1,
 * i.e. becomes active at the start of cell k. duties[cells] is not a real cell:
 * it is whatever the caller wants the wire to do once the frame is over, i.e. the
 * stop cell (duty 0: never low) for a write frame and the first response clock
 * (LOW1) for a read one.
 *
 * One store per cell is what the CPU can afford now that the edges are the
 * timer's: 9 cycles against a cell of 86 ticks, and a read frame has nothing to
 * store at all beyond its header, since every response cell is LOW1. The boundary
 * is polled on the update flag (waitCellBoundary()), so a store that arrives late
 * shifts the cell it arms instead of dropping it: the wire stays one LOW per cell
 * whatever the CPU does.
 *
 * On return the carrier is inside cell cells-1 and duties[cells] is armed for cell
 * cells: the caller waits for that cell's boundary (waitStopCell()) or, in a read
 * frame, samples first and preloads the stop cell after its last sample.
 *
 * @return the number of boundaries this took late (waitCellBoundary()): a store
 *         that late still lands before the next boundary in a read frame's short
 *         header, i.e. it shifts one cell's *duty* by a cell rather than moving an
 *         edge, so the count is a diagnostic about the frame's data rather than
 *         about its clock - and the caller is the one that knows which frame it
 *         was.
 */
int SdiTimer::runCells(const uint16_t *duties, int cells)
{
    int late = 0;

    xAssert(cells > 0);
    _t->CNT = 0;
    _t->CHCVs[_channel] = duties[0];
    _t->SWEV |= LN_TIMER_SWEVG_UPG;  // commit cell 0's duty: it starts with CEN
    _t->CHCVs[_channel] = duties[1]; // shadowed: cell 1's duty, active at its start
    _t->INTF = 0;
    _t->CTL0 |= LN_TIMER_CTL0_CEN; // cell 0 begins here
    for (int i = 2; i <= cells; i++)
    {
        if (waitCellBoundary()) // the boundary that starts cell i-1
            late++;
        _t->CHCVs[_channel] = duties[i];
    }
    return late;
}

/**
 * @brief Wait for the boundary that starts the stop cell, i.e. the end of the
 *        frame's last real cell.
 *
 * The channel output is high from that boundary on (the stop cell's duty is 0,
 * never low) and stays there, so the caller can stop the carrier and hand the pad
 * back before the wire could move again.
 */
void SdiTimer::waitStopCell()
{
    waitCellBoundary();
}

/**
 * @brief Stop the carrier. The channel output stays where it is, i.e. high.
 *
 * The stop cell's duty is 0 (never low), so CEN can be cleared anywhere in the
 * cell without moving the wire; the counter is reset so that the next frame's t0
 * is the same tick in every frame.
 *
 * CHxEN is *not* cleared: PB8 only reaches the timer through the alternate
 * function, so an enabled channel cannot touch the wire while the pad is a GPIO,
 * and leaving it enabled is what makes frames 2..n work - the pad is switched
 * back to the alternate function before the next header's first duty is
 * committed, and the channel has to be able to make that first falling edge from
 * there.
 */
void SdiTimer::stopCarrier()
{
    _t->CTL0 &= ~LN_TIMER_CTL0_CEN;
    _t->CNT = 0;
}

/** @brief Wait until the counter reaches @p tick inside the current cell. */
void SdiTimer::waitTick(uint32_t tick)
{
    while (_t->CNT < tick)
        ;
}

/**
 * @brief Wait for the next cell boundary, and never drop a cell.
 *
 * The counter wraps once per cell and the update flag latches that rollover, so
 * a boundary that arrived while the CPU was busy is *taken late* instead of being
 * skipped - and now that the timer makes every edge of a read frame (see
 * sdiSampleWord()) "late" only means what the CPU does with the boundary moves a
 * few ticks later inside the same cell, never that a clock edge is missing. That
 * matters: a dropped clock edge is a wrong clock (the target's bit counter has no
 * way to know, and its response to the dropped clock is never sampled), while a
 * late sample still reads the bit the target is holding until the next falling
 * edge.
 *
 * Every caller comes here straight from the previous boundary, so the flag is
 * clear on entry and every call waits a full cell: nothing has to be cleared
 * first.
 *
 * @return true when the boundary was already late, i.e. the CPU did not keep up
 *         with the cells. A diagnostic only: the target sees no difference.
 */
bool SdiTimer::waitCellBoundary()
{
    const bool late = (_t->INTF & LN_TIMER_INTF_UPIF) != 0;
    if (!late)
    {
        while (!(_t->INTF & LN_TIMER_INTF_UPIF))
            ;
    }
    _t->INTF = 0;
    return late;
}
// ---------------------------------------------------------------------------
// Pin role, cell builders and the two frame generators.
// ---------------------------------------------------------------------------
static SdiTimer *sdiTimer = NULL; // created on entering SDI mode
static bool sdiMode = false;      // SDI owns PB8 (PC3 is never driven - SDI_LN_DRIVE_DIR_PIN)
/* Cells the last read frame took late, and whether that has been reported: the
 * only in-firmware evidence that the CPU timed cells fit in a cell time. */
static uint32_t sdiRxLateCells = 0;
static bool sdiRxLateReported = false;

/**
 * @brief Hand PB8 to the timer channel (alternate function), channel output high.
 *
 * The output register is set before the pad changes role, and the channel's duty
 * is still 0 at this point (its output is high), so the wire is high before and
 * after: the hand-over itself produces no edge. The first falling edge of the
 * frame is the duty runCells() commits right after, and the shifter needs nothing
 * here: SDI never drives PC3, so the direction is the same for the whole session and
 * for whatever tap ran before it (SDI_LN_DRIVE_DIR_PIN).
 */
static void sdiPinToTimer()
{
    rSWDIO->_fast.on(); // keep the ODR bit consistent for the way back
    lnPinMode(SDI_DATA_PIN, lnPWM, SWD_IO_SPEED);
}

/**
 * @brief Hand PB8 to the timer channel as an *open drain* AF output (read frames).
 *
 * A read frame's cells are made by the channel exactly like a write's (one
 * compare event per cell, CHCV = the cell's LOW time), but the release has to be
 * a high-Z: the target drives the wire during the second half of every response
 * cell, and with the pad still driving (AF push-pull, the write's mode) the probe
 * would read its own level instead. `lnALTERNATE_OD` keeps the channel as the
 * source of the LOW and leaves the HIGH to the level shifter, i.e. exactly the
 * pair of levels a CPU made read cell produced by hand.
 *
 * The release is necessary but not sufficient, and what is missing is the shifter's
 * direction: it stops the *pad* from driving, but while PC3 points the probe's way
 * the shifter keeps passing the probe's (pulled-up) level on to the target, so the
 * pad would read its own side of the shifter instead of the target. The answer here
 * is to not drive PC3 at all (SDI_LN_DRIVE_DIR_PIN), i.e. to let the board decide
 * which way the shifter sits; the per-cell turn-around this tap used to make in
 * sdiSampleWord() (DIR = probe for the LOW, DIR = target from the release on) is
 * compiled out. The RP2040 tap never needed one: it turns the wire around per bit
 * with its PIO pin direction, with the direction pin written once in setupSDI().
 */
static void sdiPinToTimerOpenDrain()
{
    lnPinMode(SDI_DATA_PIN, lnALTERNATE_OD, SWD_IO_SPEED);
}

/**
 * @brief Give PB8 back as an open drain, released.
 *
 * The channel output is high (the stop cell) and the output register is set
 * before the pad stops being driven by the timer, so the wire goes from
 * driven-high to pulled-high. From here on the pad can pull the wire low
 * (switch()) and read it (read()).
 */
static void sdiPinToOpenDrain()
{
    rSWDIO->_fast.on();                              // ODR = 1: released
    lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, 1); // open drain, latch or sense
}

/** @brief The LOW time that carries @p bit: 1 is short, 0 is long (§2.2). */
static inline uint16_t sdiDutyForBit(uint32_t bit)
{
    return bit ? (uint16_t)sdiTimer->low1() : (uint16_t)sdiTimer->low0();
}

/** @brief Fill @p duties with the nine header cells, start bit first. */
static int sdiBuildHeader(uint16_t *duties, uint8_t adr, uint8_t mode)
{
    const uint32_t header = make_sdi_header(adr, mode);
    for (int i = 0; i < SDI_HEADER_BITS; i++)
        duties[i] = sdiDutyForBit((header >> (SDI_HEADER_BITS - 1 - i)) & 1U);
    return SDI_HEADER_BITS;
}

/** @brief Fill @p duties with the 32 payload cells, MSB first (sdi.pio order). */
static int sdiBuildWord(uint16_t *duties, uint32_t data)
{
    for (int i = 0; i < SDI_WORD_BITS; i++)
        duties[i] = sdiDutyForBit((data >> (SDI_WORD_BITS - 1 - i)) & 1U);
    return SDI_WORD_BITS;
}

/**
 * @brief Send one frame of @p cells cells and leave the wire idling high.
 *
 * duties[0..cells-1] are the real cells, contiguous cell by cell (the cell clock
 * never stops inside a frame); duties[cells] is the stop cell, i.e. duty 0 =
 * never low, so the wire is already high when waitStopCell() returns and the pad
 * can go back to the GPIO. A write passes its 41 cells in one call, see
 * sdiWriteFrame(); a read cannot (it samples *while* its carrier runs, see
 * sdiReadFrame()).
 *
 * The late count is not reported here: this path stores one duty per cell and
 * does nothing else, so a boundary taken late would be the first sign of a CPU
 * that is out of time altogether, and the read path is where that shows up.
 */
static void sdiSendFrame(uint16_t *duties, int cells)
{
    duties[cells] = 0; // stop cell: never low, so the frame ends with the wire high
    sdiPinToTimer();
    (void)sdiTimer->runCells(duties, cells);
    sdiTimer->waitStopCell(); // the wire is high from here on
    sdiTimer->stopCarrier();
    sdiPinToOpenDrain();
}

/**
 * @brief The pad's level, without lnDigitalRead()'s port lookup and call.
 *
 * A response cell has 26 ticks of slack after its sample (tick 60 to the next
 * boundary at 86, and 48 before that boundary's window closes), and the call was
 * most of them: this is the single load SwdPin::read() would end up doing anyway.
 */
static inline LN_ALWAYS_INLINE bool sdiReadPad()
{
    return rSWDIO->_fast.read();
}

/**
 * @brief Sample the 32 response cells the carrier is clocking out, MSB first.
 *
 * The cells are the timer's - the LOW and the release of every one of them - and
 * what is left for the CPU is three things per cell, in this order:
 *
 *   waitRelease()     the carrier lets go of the wire at LOW1, i.e. the read
 *                     clock's LOW is over,
 *   sdiDirTarget()    where this tap used to hand the wire to the target through the
 *                     shifter, so that the level the target drives is what the pad
 *                     sees (the pad's own release does not do it - see
 *                     sdiPinToTimerOpenDrain()); on LN the direction pin is not
 *                     driven at all (SDI_LN_DRIVE_DIR_PIN), so this is nothing,
 *   waitSampleTick()  the cell's sample point, where sdiReadPad() reads the bit,
 *   sdiDirProbe()     back to the probe, before the next read clock's falling edge
 *                     - likewise nothing on LN.
 *
 * Four bus accesses per cell on LN (the boundary, the two awaits and the pad read),
 * against the ~10 a hand made cell cost, and the two waits hold the CPU at the
 * ticks that matter. The sample tick is absolute, so a
 * boundary taken late moves the *tail* work later but never the sample into the
 * next cell: the target holds its bit until the next falling edge (sdi.txt §2:
 * data windows are (T,2T) LOW and (T,8T) HIGH), so anywhere between 500 ns and
 * the end of the cell reads the same bit. The tail (`period - sample`, 26 ticks
 * here) is what the CPU has to fit its read - and the two DIR stores, when
 * SDI_LN_DRIVE_DIR_PIN is 1 - into; when it does
 * not, sdiRxLateCells counts it and the sample still lands at the same tick.
 *
 * The call ends by preloading the stop cell, so the caller's waitStopCell() is a
 * plain boundary wait: from the end of the last response cell (cell 40) on, the
 * carrier's duty is 0 and the wire stays high.
 */
/** @brief Level shifter -> probe: the pad's level reaches the target.
 *
 *  Nothing on this platform: DIR is not driven here (SDI_LN_DRIVE_DIR_PIN), so the
 *  shifter's direction register does not appear in the read path at all; the two call
 *  sites in sdiSampleWord() stay as the record of the sequence this tap used to need. */
static inline LN_ALWAYS_INLINE void sdiDirProbe()
{
#if SDI_LN_DRIVE_DIR_PIN
    rSWDIO->_fastdir.on();
#endif
}
/** @brief Level shifter -> target: the probe lets go, and reads what it drives. */
static inline LN_ALWAYS_INLINE void sdiDirTarget()
{
#if SDI_LN_DRIVE_DIR_PIN
    rSWDIO->_fastdir.off();
#endif
}

static uint32_t sdiSampleWord()
{
    uint32_t word = 0;

    /* One boundary, and it is the first response cell's: runCells() stopped inside
     * the last header cell (cell 8) with cell 9's duty - the first read clock -
     * already armed, so this wait is what starts that clock. Nothing has to be
     * prepared for it: DIR is not driven on this platform, so the direction the
     * header ran with is the direction the response runs with. */
    if (sdiTimer->waitCellBoundary())
        sdiRxLateCells++;

    for (int i = 0; i < SDI_WORD_BITS; i++)
    {
        // cell i's boundary is the one above (i == 0) or the previous iteration
        // (i > 0) has just taken
        if (i && sdiTimer->waitCellBoundary())
            sdiRxLateCells++;
        sdiTimer->waitRelease(); // the LOW is over: the carrier released the wire
        sdiDirTarget();          // no-op on LN; the target owns the wire until the next falling edge
        sdiTimer->waitSampleTick();
        word = (word << 1) | (sdiReadPad() ? 1U : 0U);
        sdiDirProbe(); // back to the probe, before the next read clock
    }
    sdiTimer->preload(0); // the stop cell: the wire stays high from its boundary on
    return word;
}


// ---------------------------------------------------------------------------
// Frame level: the SDI transport (same wire format as sdi_write / sdi_read in
// bmp_sdiTap_rp2040.cpp).
// ---------------------------------------------------------------------------
/**
 * @brief Send one DM write: the nine header cells and the 32 payload cells as
 *        *one* contiguous packet, then idle high.
 *
 * The packet is a single 41-cell unit (§2.1: start bit, 7 address bits, R/W, 32
 * data bits): a HIGH of 10 T or more *is* a stop bit (§2.2), so splitting the
 * header and the payload into two frames - which is what a gap between the two
 * sdiSendFrame() calls amounts to - ends the packet after the header and the
 * payload is then parsed as a packet of its own. That is only accidentally
 * right for a payload whose MSB is 0, where it forms a §2.1 *bypass packet*
 * ("1 bit start bit, fixed as data 0 ... read/write bits and register addresses
 * are the same as the most recent New Packet transfer"): 0x5AA50400 goes
 * through that way, while a payload whose MSB is 1 (dmcontrol = 0x80000001, any
 * value with bit 31 set) becomes a bogus new packet - start bit 1, address
 * bits [30:24], R/W = bit 23 - and the write is silently dropped. sdi.pio is
 * continuous across the two words for the same reason: its second `pull block`
 * already has the payload in the FIFO by the time the header's nine cells are
 * out, so the state machine never stalls between them.
 *
 * The 55 us that follow are the §2.2 stop/idle (>= 10 T) before the next write.
 */
static void sdiWriteFrame(const uint8_t adr, const uint32_t data)
{
    uint16_t duties[SDI_HEADER_BITS + SDI_WORD_BITS + 1]; // 41 cells + the stop cell
    int cells = sdiBuildHeader(duties, adr, SDI_WRITE_FLAG);

    cells += sdiBuildWord(duties + cells, data);
    sdiSendFrame(duties, cells);
    lnDelayUs(END_OF_WRITE_DELAY);
    /* One line per write, so the *shape* of the packet is checkable without a
     * scope: "one packet" is exactly what the two-frame write of §12 could not
     * print, and a firmware whose console has no such line is a firmware whose
     * write path is not this one. Reported after the §2.2 stop delay, so it
     * cannot shorten the write's idle. Single Logger call on purpose: the
     * output path drains one shared buffer, so two back-to-back calls can drop
     * the first line. */
    Logger("SDI : write 0x%02x = 0x%x : one packet, %d cells\n", (unsigned)adr, (unsigned)data, cells);
}

/**
 * @brief Send one DM read: the nine header cells and the 32 response cells as
 *        one carrier frame, sampling the response while it runs.
 *
 * Structurally the same frame a write uses - one compare event per cell, CHCV =
 * the cell's LOW time, the duties fed by the CPU a boundary ahead - with two
 * differences, and they are what makes a read possible at all:
 *
 *  - the pad goes to the channel in *open drain* (sdiPinToTimerOpenDrain()), so
 *    the HIGHs are a release: the target drives the wire for the second half of
 *    every response cell and the probe reads it there,
 *  - the level shifter turns once per response cell on the LN tap that drove it (DIR =
 *    probe for the read clock's LOW, DIR = target from the release to the sample, see
 *    sdiSampleWord()). This build does not drive that pin at all - see
 *    SDI_LN_DRIVE_DIR_PIN - so the only state a read frame needs on the wire is the
 *    pad's own open drain. (The RP2040 tap needs the same wire turnaround, but it gets
 *    it from the PIO's pin direction, not from the shifter.)
 *
 * The header's duties are its nine bits and the one after them is LOW1: a read
 * clock carries no data of its own (§2.2 - the probe's LOW is the clock and the
 * target's level during the HIGH is the bit), so all 32 response cells have the
 * same duty and the CPU stores no duty for cells 10..40 at all. What it does
 * there is wait for each boundary and read the pad (sdiSampleWord()), and the frame
 * is closed by preloading the stop cell right
 * after the last sample: cell 41 is the first one with duty 0, i.e. the wire's
 * return to idle.
 *
 * Interrupts stay masked, as they were for the CPU timed frame: the CPU only
 * samples now, but an ISR inside a cell would still push a sample past the
 * boundary, and the scan runs reads back to back. The Logger call stays outside
 * the masked section.
 */
static uint32_t sdiReadFrame(const uint8_t adr)
{
    uint16_t duties[SDI_HEADER_BITS + 1]; // the nine header cells + the first read clock
    uint32_t word;
    int cells;

    sdiRxLateCells = 0;
    lnNoInterrupt();

    cells = sdiBuildHeader(duties, adr, SDI_READ_FLAG);
    duties[cells] = (uint16_t)sdiTimer->low1(); // cell 9, the first response clock

    sdiPinToTimerOpenDrain();
    sdiRxLateCells = (uint32_t)sdiTimer->runCells(duties, cells);
    word = sdiSampleWord();
    sdiTimer->waitStopCell(); // the stop cell: the wire is high from here on
    sdiTimer->stopCarrier();
    sdiPinToOpenDrain();
    lnInterrupts();

    /* Reported outside the masked section, and only the first time it happens.
     * The timer makes the cells either way, so this is a late *sample* or a late
     * duty store, not a wrong clock: worth one line when it happens - it means the
     * CPU is too slow for the cells and a response bit may have been read late -
     * and silence when it does not. Per frame it cannot print: the scan runs reads
     * back to back and a Logger call is longer than the read it would report. */
    if (sdiRxLateCells && !sdiRxLateReported)
    {
        sdiRxLateReported = true;
        Logger("SDI : read 0x%02x : %u of %u boundaries taken late (the timer makes the cells; only the sample is late)\n",
               (unsigned)adr, (unsigned)sdiRxLateCells, (unsigned)(SDI_HEADER_BITS + SDI_WORD_BITS));
    }

    rSWDIO->_fast.on(); // released; PB8 is left as the taps release it, and PC3 is untouched
    lnDelayUs(INTER_WORD_DELAY);
    return word;
}

// ---------------------------------------------------------------------------
// Pin mode side: ownership of PB8/PC3 for the duration of an SDI session. Called
// by bmp_gpio_pinmode() in bmp_tap_ln.cpp, the LN platform file that knows every
// mode (mirroring bmp_tap_rp2040.cpp on the RP2040 side).
// ---------------------------------------------------------------------------
/**
 * @brief Enter SDI mode: build the cell engine and park PB8 released.
 *
 * PB8 leaves the tap in open drain and released, which is the state every frame
 * starts and ends in. PC3, the shifter's direction, is not touched here or anywhere
 * else in the SDI path (SDI_LN_DRIVE_DIR_PIN).
 */
void sdi_pinmode_enter()
{
    if (!rSWDIO)
        return;
    if (!sdiTimer)
    {
        sdiTimer = new SdiTimer(SDI_DATA_PIN);
        sdiTimer->configure();
        sdiTimer->logTicks();
    }
    sdiRxLateReported = false; // one late-cell report per SDI session
    if (!sdiMode)
    {
        rSWDIO->_fast.on(); // released
#if SDI_LN_DRIVE_DIR_PIN
        rSWDIO->_fastdir.on(); // DIR = probe
#endif
        lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, 1);
        Logger("SDI : SDI mode on pin %d\n", (int)SDI_DATA_PIN);
    }
    sdiMode = true;
}

/**
 * @brief Leave SDI mode: stop the carrier and give the pad back released.
 *
 * PB8 is left in the state the SWD/RVSWD taps start from (released open drain, forced
 * high by SwdPin::on()), so the next swdptap_init()/rvswd_scan() takes over without a
 * fight on the wire. PC3 was never driven by the SDI path, and both taps set the
 * shifter's direction in their own constructor anyway.
 */
void sdi_pinmode_leave()
{
    if (!sdiMode)
        return;
    sdiMode = false;
    if (sdiTimer)
        sdiTimer->stopCarrier();
    if (rSWDIO)
    {
        rSWDIO->_fast.on(); // released
#if SDI_LN_DRIVE_DIR_PIN
        rSWDIO->_fastdir.on(); // DIR = probe
#endif
        lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, 1);
    }
    Logger("SDI : leaving SDI mode\n");
}

// EOF