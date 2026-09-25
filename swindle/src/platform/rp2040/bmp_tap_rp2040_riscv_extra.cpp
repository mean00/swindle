/**
 * @file bmp_tap_rp2040_riscv_extra.cpp
 * @brief Pin hand-over for the WCH SDI mode (RP2040): the PIO program upload that
 *        used to sit in bmp_tap_rp2040.cpp.
 *
 * SDI is optional and is a leaf of the RISC-V support, so its side of the pin
 * mode switch lives in this file, next to the transport it configures
 * (bmp_sdiTap_rp2040_riscv_extra.cpp). Everything the rest of the firmware sees
 * is the two hooks declared in swindle/include/bmp_riscv_extra.h, which
 * bmp_tap_rp2040.cpp calls and src/template/bmp_riscv_extra_stubs.cpp replaces
 * with no-ops when SWINDLE_WITH_SDI=OFF.
 */
#include "esprit.h"
#include "lnGPIO.h"
#include "ln_rp_pio.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "lnBMP_pins.h"
#include "lnRP2040_pio.h" /* LN_SWD_PIO_ENGINE (the shared SWD/RVSWD/SDI engine) */
#include "bmp_pio_sdi.h"
#include "bmp_riscv_extra.h"

/* The shared SWD/RVSWD/SDI state machine, owned by bmp_tap_rp2040.cpp. */
extern rpPIO_SM *xsm;

/**
 * @brief Upload and configure the single-wire SDI PIO program (WCH CH32V0xx).
 *
 * SDI is a one-wire pseudo open-drain protocol: only the shared SWDIO line is
 * used, there is no clock or direction side-set. The line idles high through
 * the pad pull-up and the PIO program pulls it low for each bit. When SDI is
 * selected after a SWD/RVSWD session the clock/direction pads are released
 * from PIO function so they do not float while the SWDIO pad stays on PIO.
 */
/* Requested SDI state-machine clock. rpPIO_SM::setSpeed() floors the divider
 * (intdiv = clk_sys / fq, see ln_rp_pio.cpp), so with clk_sys = 125 MHz
 * (LN_MCU_SPEED) this request for 16 MHz yields CLKDIV = 7 -> 17.857 MHz
 * (56 ns/cycle). That is what the sdi.pio bit schedule depends on: its 4-cycle
 * logic-1 low comes out at 224 ns, just under the fast-1x ceiling of 2T = 250 ns
 * (a /8 divider, 15.625 MHz, would stretch it to 256 ns and break the window).
 * Do not round this request up or change clk_sys without re-checking sdi.pio,
 * which carries the window table and the ~17-26 MHz valid band. */
#define SDI_PIO_FREQUENCY_HZ 16000000U
static void setupSDI()
{
    lnPin pin_sdi = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    lnPin pin_dir = _mapping[TDIRECTION_PIN];

    xsm->reset();
    lnDigitalWrite(pin_clk, 1);
    lnPinMode(pin_clk, lnOUTPUT);
    lnDigitalWrite(pin_dir, 1);
    lnPinMode(pin_dir, lnOUTPUT);

    lnPinModePIO(pin_sdi, LN_SWD_PIO_ENGINE, true); // PIO0 + pull-up (idle high)
    xsm->setSpeed(SDI_PIO_FREQUENCY_HZ);
    xsm->setBitOrder(false, false); // MSB-first: WCH SDI frames start at bit 31

    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_sdi;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_sdi;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_sdi;
    xsm->uploadCode(sizeof(sdi_program_instructions) / 2, sdi_program_instructions, sdi_wrap_target, sdi_wrap);
    xsm->configure(pinConfig);

    // Pseudo open-drain baseline: output level 0 while the direction floats.
    // pindirs=1 then drives a 0 (line low); pindirs=0 floats back high through
    // the pad pull-up.
    xsm->setPinsValue(0);
    xsm->setPinDir(pin_sdi, false);
    xsm->execute();
}

/**
 * @brief Pin hand-over for the SDI mode, called by bmp_gpio_pinmode().
 *
 * The upload itself is the only thing SDI needs from the pin mode switch: the
 * program drives the single wire from the FIFO, so there is nothing to hand back
 * on the way out - the next bmp_gpio_pinmode() reprograms the pads anyway.
 */
void sdi_pinmode_enter()
{
    setupSDI();
}

/**
 * @brief Leave the SDI mode: nothing to do (the next pin mode reprograms the pads).
 */
void sdi_pinmode_leave()
{
}

// EOF
