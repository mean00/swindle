/**
 * @file bmp_sdiTap_rp2040.cpp
 * @brief WCH SDI (single wire debug interface) tap: RP2040 PIO transport.
 *
 * The SDI protocol itself (DM/configuration registers, §2.4 shadow/commit,
 * frame format, CPBR diagnostics, DMI glue and the RISC-V attach) is shared with
 * the other platforms in ../../template/sdi_template.h. This file is only the
 * wire: bmp_gpio_pinmode(BMP_PINMODE_SDI) uploads the sdi.pio program onto the
 * shared state machine (bmp_tap_rp2040.cpp setupSDI()) and one whole DM frame is
 * one word pushed into / pulled out of its FIFO.
 */
#include "lnGPIO.h"
#include "ln_rp_pio.h"
#include "stdint.h"
// Only the instruction array / wrap constants are needed from the pioasm
// output (no hardware/pio.h structs): esprit drives the PIO directly.
#define PICO_NO_HARDWARE 1
#include "bmp_pio_sdi.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
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
}
extern "C" void target_list_free(void);

// SDI data pin: the same physical wire the PIO state machine is configured on
// by bmp_gpio_pinmode(BMP_PINMODE_SDI) (bmp_tap_rp2040.cpp), i.e. the SWDIO
// line _mapping[TSWDIO_PIN] (GPIO13 on the RP2040 carrier/inv probe, where the
// primitives were validated). NRST sits on GPIO10 there but is driven through
// the shared SwdReset controller so the build-time polarity option is honored.
#define SDI_DATA_PIN _mapping[TSWDIO_PIN]

// ---------------------------------------------------------------------------
// Platform transport: one whole DM frame as a PIO FIFO word.
//
// The hooks below are the only thing sdi_template.h needs from this platform.
// They are macros, not functions, so the frame-level code in the template
// expands to exactly the same source the tap used before the protocol layer was
// moved out (-ffunction-sections: per-function codegen is untouched).
// ---------------------------------------------------------------------------
extern rpPIO_SM *xsm;    // shared SWD/RVSWD/SDI PIO state machine (bmp_tap_rp2040.cpp)
extern SwdReset *pReset; // board NRST controller (bmp_tap_rp2040.cpp)

/* Frame primitives, defined at the bottom of this file: a whole frame is one
 * word in (header + payload) and one word out, with the PIO doing the bit
 * timing. */
static uint32_t sdi_read(rpPIO_SM *xsm, const uint8_t adr);
static void sdi_write(rpPIO_SM *xsm, const uint8_t adr, const uint32_t data);

#define sdiFrameRead(adr) sdi_read(xsm, (adr))
#define sdiFrameWrite(adr, val) sdi_write(xsm, (adr), (val))
/* §2.4(3): pull the wire low for @p ms, then let it float high again. The PIO
 * output value is already 0 here, so the direction bit alone drives the pulse. */
#define sdiWireHoldLow(ms)                   \
    do                                       \
    {                                        \
        xsm->setPinDir(SDI_DATA_PIN, true);  \
        lnDelayMs(ms);                       \
        xsm->setPinDir(SDI_DATA_PIN, false); \
    } while (0)

/* The shared protocol layer (DMSTATUS/DMACCESS handling, §2.4 configuration,
 * sdi_dm_start/write/read, bmp_sdi_dm_*_c, ch32_sdi_dmi_read/write, sdi_scan).
 * Requires: bmp_pinmode.h, pReset, Logger/lnDelayMs/lnDelayUs, <cstring> and
 * the blackmagic RISC-V target framework, all included above. */
#include "sdi_template.h"

// ---------------------------------------------------------------------------
static uint32_t sdi_read(rpPIO_SM *xsm, const uint8_t adr)
{
    // Word 1: Mode=0 (bit 31 is the flag consumed by the PIO, not transmitted),
    // header (bits 30 down to 22)
    uint32_t header9 = make_sdi_header(adr, SDI_READ_FLAG);
    uint32_t w = (0 << 31) | (header9 << 22);
    xsm->write(1, &w);
    lnDelayUs(INTER_WORD_DELAY);

    xsm->read(1, &w);
    lnDelayUs(INTER_WORD_DELAY);
    return w;
}

static void sdi_write(rpPIO_SM *xsm, const uint8_t adr, const uint32_t data)
{
    // Word 1: Mode=1 (bit 31 is the flag consumed by the PIO, not transmitted),
    // header (bits 30 down to 22)
    uint32_t header9 = make_sdi_header(adr, SDI_WRITE_FLAG);
    uint32_t w1 = (1U << 31) | (header9 << 22);
    xsm->write(1, &w1);
    lnDelayUs(INTER_WORD_DELAY);

    // Word 2: The exact 32-bit payload
    xsm->write(1, &data);
    lnDelayUs(END_OF_WRITE_DELAY);
}

// EOF