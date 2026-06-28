/**
 * @file bmp_swdTap_ln.cpp
 * @brief Low-level SWD bit-bang I/O implementation over esprit GPIO.
 *
 * Derived from Blackmagic Probe SWD tap code.
 */
#include "bmp_pinout.h"
#include "esprit.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"
#include "bmp_swdio_ln.h"

#include "bmp_tap_ln.h"
#include "lnbmp_parity.h"

extern void gmp_gpio_init_adc();
extern "C" void swdptap_init_stubs();

/**
 * @brief Initialise the SWD TAP.
 */
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
    rSWDIO->output();
}

#define SWINDLE_FAST_IO
#define SWD_WAIT_PERIOD() swait()
#define DIR_INPUT() rSWDIO->input()
#define DIR_OUTPUT() rSWDIO->output()

/**
 * @brief Write @p nbTicks bits LSB-first on SWDIO.
 * @param nbTicks Number of bits to write.
 * @param val     Bit values (only low @p nbTicks bits are used).
 */
static void zwrite(uint32_t nbTicks, uint32_t val)
{
    xAssert(rSWDIO->dir());
    for (int i = 0; i < nbTicks; i++)
    {
        rSWDIO->set(val & 1);
        rSWCLK->invPulseClock();
        val >>= 1;
    }
}

/**
 * @brief Read @p nbTicks bits LSB-first from SWDIO.
 * @param nbTicks Number of bits to read.
 * @return Bits sampled, LSB first.
 */
static uint32_t zread(uint32_t nbTicks)
{
    xAssert(!rSWDIO->dir());
    uint32_t index = 1;
    uint32_t val = 0;
    for (int i = 0; i < nbTicks; i++)
    {
        uint32_t bit = rSWDIO->read();
        rSWCLK->invPulseClock();
        if (bit)
            val |= index;
        index <<= 1;
    }
    return val;
}

/**
 * @brief Stub: set SWD pin mode (not implemented for LN targets).
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
    (void)pioMode;
}

/**
 * @brief Drive SWDIO high and set to output (reset line state).
 */
extern "C" void bmp_gpio_reset()
{
    rSWDIO->set(1);
    rSWDIO->output();
}

/**
 * @brief Bit-bang an SWD reset sequence (at least 50 clock cycles high).
 * @param pulses Number of clock cycles to hold SWDIO high.
 */
extern "C" void ln_raw_swd_reset(uint32_t pulses)
{
    rSWDIO->set(1);
    rSWDIO->output();
    for (int i = 0; i < pulses; i++)
        rSWCLK->invPulseClock();
}
#include "swd_template.h"
// EOF
