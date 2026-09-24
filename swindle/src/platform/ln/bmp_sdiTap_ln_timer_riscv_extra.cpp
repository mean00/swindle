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

#define SDI_TBIT_NS 1200u // was 1000u
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

static inline LN_ALWAYS_INLINE void sdiOutput()
{
    rSWDIO->_fastdir.on();
}
static inline LN_ALWAYS_INLINE void sdiInput()
{
    rSWDIO->_fastdir.off();
}
static inline LN_ALWAYS_INLINE void sdiPadDriven()
{
    lnPinMode(SDI_DATA_PIN, lnOUTPUT, SWD_IO_SPEED);
}
static inline LN_ALWAYS_INLINE void sdiPadOpenDrain()
{
    lnPinMode(SDI_DATA_PIN, lnOUTPUT_OPEN_DRAIN, 1);
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

static void sdiPark()
{
    sdiPadRelease();
    sdiOutput();
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
        sdiOutput();
        sdiPadLow();
        sdi_watch->wait(sdi_ticks_low1);
        sdiPadRelease();
        sdiInput();
        sdi_watch->wait(sdi_ticks_sample);
        word = (word << 1) | (sdiReadPad() ? 1U : 0U);
        sdi_watch->wait(sdi_ticks_tbit);
    }
    return word;
}

static void sdiWriteFrame(const uint8_t adr, const uint32_t data)
{
    lnNoInterrupt();
    sdiPadDriven();
    sdiOutput();

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
    sdiPadDriven();
    sdiOutput();

    sdiSendHeader(adr, SDI_READ_FLAG);

    sdiPadOpenDrain();
    word = sdiSampleWord();

    sdiPadRelease();
    sdiOutput();
    lnInterrupts();

    return word;
}
