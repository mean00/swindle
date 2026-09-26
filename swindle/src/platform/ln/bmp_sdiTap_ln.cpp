/**
 * @file bmp_sdiTap_ln_timer_riscv_extra.cpp
 * @brief WCH SDI tap for the LN platform: Timer-polled (lock-free hardware timebase) bit-bang transport.
 */

#include "lnGPIO.h"
#include "stdint.h"
#include "esprit.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_tap_ln.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
#include <cstring>
#include "bmp_riscv_extra.h"
#include "lnTimerWatch.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
    extern "C" void target_list_free(void);

    extern "C" bool bmp_mem_log_enabled;

#define SDI_DATA_PIN _mapping[TSWDIO_PIN]

    static void sdiWriteFrame(const uint8_t adr, const uint32_t data);
    static uint32_t sdiReadFrame(const uint8_t adr);

#define sdiFrameWrite(adr, val) sdiWriteFrame((adr), (val))
#define sdiFrameRead(adr) sdiReadFrame(adr)
#define sdiWireHoldLow(ms)                                                                                             \
    do                                                                                                                 \
    {                                                                                                                  \
        rSWDIO->_fastdir.on(); /* DIR = probe */                                                                       \
        rSWDIO->_fast.off();   /* Drive LOW */                                                                         \
        lnDelayMs(ms);                                                                                                 \
        rSWDIO->_fast.on(); /* Release to float HIGH */                                                                \
    } while (0)

#include "sdi_template.h"
}

#define SDI_TBIT_NS 1200u
#define SDI_LOW1_NS 250u
#define SDI_LOW0_NS 900u
#define SDI_RX_SAMPLE_NS 625u
#define TIMER_TO_USE 4 // Remember then our timer starts at 0, not 1 so this is timer6 for STM32

static lnTimerWatch *sdi_watch = NULL;
static uint32_t sdi_ticks_per_us = 0;

static uint32_t sdi_ticks_low1;
static uint32_t sdi_ticks_low0;
static uint32_t sdi_ticks_sample;
static uint32_t sdi_ticks_tbit;
static bool sdiTimingDone = false;
static bool sdiMode = false;
static bool sdiEntryLogged = false;

struct SdiWire
{
    uint32_t tbitNs, low1Ns, low0Ns, sampleNs;
};

static SdiWire sdiWireUsed = {0u, 0u, 0u, 0u};

static SdiWire sdiWireRead()
{
    SdiWire w;
    w.tbitNs = bmp_sdi_wire_tbit_ns;
    w.low1Ns = bmp_sdi_wire_low1_ns;
    w.low0Ns = bmp_sdi_wire_low0_ns;
    w.sampleNs = bmp_sdi_wire_sample_ns;
    return w;
}

static bool sdiWireChanged()
{
    const SdiWire w = sdiWireRead();
    return w.tbitNs != sdiWireUsed.tbitNs || w.low1Ns != sdiWireUsed.low1Ns || w.low0Ns != sdiWireUsed.low0Ns ||
           w.sampleNs != sdiWireUsed.sampleNs;
}

static uint32_t sdiWireValue(uint32_t overrideVal, uint32_t defaultVal)
{
    return overrideVal ? overrideVal : defaultVal;
}

static void sdiInitTimer()
{
    if (!sdi_watch)
    {
        sdi_watch = new lnTimerWatch(TIMER_TO_USE); // Use TIMER4 which maps to TIM5

        Peripherals per = (Peripherals)(pTIMER0 + TIMER_TO_USE);
        uint32_t clock = lnPeripherals::getClock(per);
        sdi_ticks_per_us = clock / 1000000;

        sdi_watch->setup();
    }
}

static void sdiConfigureTiming()
{
    sdiInitTimer();

    const SdiWire w = sdiWireRead();
    sdiWireUsed = w;

    const uint32_t tbitNs = sdiWireValue(w.tbitNs, SDI_TBIT_NS);
    const uint32_t low1Ns = sdiWireValue(w.low1Ns, SDI_LOW1_NS);
    const uint32_t low0Ns = sdiWireValue(w.low0Ns, SDI_LOW0_NS);
    const uint32_t sampleNs = sdiWireValue(w.sampleNs, SDI_RX_SAMPLE_NS);

    sdi_ticks_low1 = (low1Ns * sdi_ticks_per_us) / 1000u;
    sdi_ticks_low0 = (low0Ns * sdi_ticks_per_us) / 1000u;
    sdi_ticks_sample = (sampleNs * sdi_ticks_per_us) / 1000u;
    sdi_ticks_tbit = (tbitNs * sdi_ticks_per_us) / 1000u;

    sdiTimingDone = true;

    Logger("SDI : HW Timer base %u ticks/us, low1=%u, low0=%u, sample=%u, tbit=%u\n", (unsigned)sdi_ticks_per_us,
           (unsigned)sdi_ticks_low1, (unsigned)sdi_ticks_low0, (unsigned)sdi_ticks_sample, (unsigned)sdi_ticks_tbit);
}

/* ---- The data pad (PB8) and the level shifter's direction (PC3) ----------------
 *
 * The pad is an open drain for the whole SDI session. sdiPadOpenDrain() writes
 * lnOUTPUT_OPEN_DRAIN once, on pin-mode entry and on leave (both through
 * sdiPark()), and no frame and no cell ever writes the mode again: every LOW the
 * probe makes is sunk by the pad's N-MOS (sdiPadLow(), ODR = 0) and every HIGH is
 * made by the wire's pull-up once the pad lets go (sdiPadRelease(), ODR = 1 =
 * high-Z). Releasing is therefore never an input-mode switch - the pad stays an
 * output and the line's level is read while it does.
 *
 * Only the output bit changes inside a cell (sdiPadLow/sdiPadRelease write the
 * pad's BOP register, sdiReadPad() loads the *input status* register, which keeps
 * following the pin in output mode), so a cell costs two stores and a load
 * whatever the line does. Reading the output-control register instead would read
 * back our own released bit, i.e. a permanent 1 - that is the register trap of an
 * open drain read, and it is why sdiReadPad() must stay on the input status.
 *
 * The consequence worth remembering: with the pad push-pull gone, the probe-driven
 * cells (a read frame's header, a whole write frame) rise through that same
 * pull-up, so pure open drain needs the wire to have one. sdiLogEntry() reports,
 * once per boot, the released and sunk levels of the wire.
 *
 * DIR (PC3) is a separate pin and a separate concern: probe side while the probe
 * drives (a cell's LOW, the whole write path), target side from a response cell's
 * release to its sample, driven by sdiLevelShifterAsOutput() / sdiLevelShifterAsInput().
 * Only its level changes - it is an output from the board's constructor on - and it is
 * a no-op on a board without the shifter.
 */
static inline LN_ALWAYS_INLINE void sdiLevelShifterAsOutput()
{
    rSWDIO->_fastdir.on();
}
static inline LN_ALWAYS_INLINE void sdiLevelShifterAsInput()
{
    rSWDIO->_fastdir.off();
}
/* The only pin-mode write of an SDI session: open drain at the same slew rate the
 * SWD pad uses, with ODR = 1 already set by sdiPark() so the pad is released the
 * instant it becomes an output. Nothing in a frame or a cell writes it again. */
static inline LN_ALWAYS_INLINE void sdiPadOpenDrain()
{
    lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, SWD_IO_SPEED);
}
static inline LN_ALWAYS_INLINE void sdiPadLow()
{
    rSWDIO->_fast.off();
}
static inline LN_ALWAYS_INLINE void sdiPadRelease()
{
    rSWDIO->_fast.on();
}
static inline LN_ALWAYS_INLINE bool sdiReadPad()
{
    return rSWDIO->_fast.read();
}

/* ---- One-shot entry diagnostic ---------------------------------------------
 *
 * Three facts, read once per boot at pin-mode entry, that decide whether a pure
 * open drain session can work at all:
 *
 *   released    the level a *released* pad sees. It is 1 only if the wire has a
 *               pull-up; with no target attached a 0 here means the probe-driven
 *               HIGHs of a read header / write frame cannot be made at all, and the
 *               fix is hardware, not a timing knob.
 *   sunk        the level the pad pulls the wire to (one short pull, then released
 *               again). A 0 proves the pad really is an output that drives the wire
 *               - a pad left in input mode would read 1 released *and* 1 sunk, and
 *               would sink nothing, which is exactly the failure this session mode
 *               must not have.
 *   DIR         the shifter's level, i.e. whether the pad is connected to the wire
 *               at all while the two levels above are measured (its mode is set by
 *               the board's constructor; SDI only changes its value). 1 = probe
 *               side, which is where sdiPark() leaves it.
 *
 * Both levels come from the input-status register, the same one sdiReadPad() uses,
 * so the reading is the one a response cell sees. The pad's mode *nibble* is not
 * reported on purpose: esprit declares lnGetGpioDirectionRegister()/lnReadPort()
 * with a uint32_t port but defines them with an int one, so the mangled names
 * differ and the declarations in lnGPIO.h cannot be linked against. The mode needs
 * no reader anyway - sdiPadOpenDrain() is the only writer and asks for
 * lnOUTPUT_OPEN_DRAIN explicitly.
 */
static void sdiLogEntry()
{
    const bool released = sdiReadPad();

    sdiPadLow();
    lnDelayUs(1);
    const bool sunk = sdiReadPad();
    sdiPadRelease();

    Logger("SDI : entry : pad %u, released %u, sunk %u, DIR %u\n", (unsigned)SDI_DATA_PIN, released ? 1U : 0U,
           sunk ? 1U : 0U, rSWDIO->_fastdir.read() ? 1U : 0U);
}

/* The single place the pad's mode is written: let go of the wire first (ODR = 1, so
 * the mode change cannot sink it), point the shifter at the probe, then make the pad
 * an open drain and leave it that way for the whole session. */
static void sdiPark()
{
    sdiPadRelease();
    sdiLevelShifterAsOutput();
    sdiPadOpenDrain();
}

void sdi_pinmode_enter()
{
    if (!rSWDIO)
        return;
    sdiPark();

    if (!sdiTimingDone || sdiWireChanged())
    {
        sdiConfigureTiming();
        sdiPark();
        sdiTimingDone = true;
    }
    sdiMode = true;

    if (!sdiEntryLogged)
    {
        sdiLogEntry();
        sdiEntryLogged = true;
    }
}

void sdi_pinmode_leave()
{
    if (!sdiMode)
        return;
    sdiMode = false;
    if (rSWDIO)
    {
        sdiPark();
    }
}

static inline LN_ALWAYS_INLINE void sdiSendCell(const uint16_t low_ticks)
{
    sdi_watch->start();
    sdiPadLow();
    sdi_watch->wait(low_ticks);
    sdiPadRelease();
    sdi_watch->wait(sdi_ticks_tbit);
}

static inline LN_ALWAYS_INLINE void sdiSendBit(const uint32_t bit)
{
    if (bit)
        sdiSendCell(sdi_ticks_low1);
    else
        sdiSendCell(sdi_ticks_low0);
}

static void sdiSendHeader(const uint8_t adr, const uint8_t mode)
{
    const uint32_t header = make_sdi_header(adr, mode);
    for (int i = 0; i < SDI_HEADER_BITS; i++)
        sdiSendBit((header >> (SDI_HEADER_BITS - 1 - i)) & 1U);
}

static void sdiSendWord(const uint32_t data)
{
    for (int i = 0; i < SDI_WORD_BITS; i++)
        sdiSendBit((data >> (SDI_WORD_BITS - 1 - i)) & 1U);
}

static uint32_t sdiSampleWord()
{
    uint32_t word = 0;

    for (int i = 0; i < SDI_WORD_BITS; i++)
    {
        sdi_watch->start();
        sdiLevelShifterAsOutput();
        sdiPadLow();
        sdi_watch->wait(sdi_ticks_low1);
        sdiPadRelease();
        sdiLevelShifterAsInput();
        sdi_watch->wait(sdi_ticks_sample);
        word = (word << 1) | (sdiReadPad() ? 1U : 0U);
        sdi_watch->wait(sdi_ticks_tbit);
    }
    return word;
}

static void sdiWriteFrame(const uint8_t adr, const uint32_t data)
{
    lnNoInterrupt();
    sdiLevelShifterAsOutput(); // shifter -> probe; the pad is already an open drain

    sdiSendHeader(adr, SDI_WRITE_FLAG);
    sdiSendWord(data);

    sdiPadRelease();
    lnInterrupts();

    lnDelayUs(END_OF_WRITE_DELAY);
}

static uint32_t sdiReadFrame(const uint8_t adr)
{
    uint32_t word;

    lnNoInterrupt();
    sdiLevelShifterAsOutput(); // shifter -> probe for the header; the pad is already an open drain

    sdiSendHeader(adr, SDI_READ_FLAG);

    /* The pad stays an open drain across the header/response boundary: the header
     * cells are the probe's, the response cells are the target's, and only the
     * output bit and DIR change between the two. */
    word = sdiSampleWord();

    sdiPadRelease();
    sdiLevelShifterAsOutput();
    lnInterrupts();

    return word;
}
