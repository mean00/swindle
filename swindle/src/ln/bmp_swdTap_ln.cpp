/*
 * Implement the low level SWD I/O on top of esprit.
 * Strongly derived from the code (C) Blackmagic Sphere
 */
#include "esprit.h"
#include "bmp_pinout.h"
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
/*
 *
 */
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
    rSWDIO->output();
}
/*
 *
 */

#define SWINDLE_FAST_IO
#define SWD_WAIT_PERIOD() swait()
#define DIR_INPUT() rSWDIO->input()
#define DIR_OUTPUT() rSWDIO->output()

/*
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
/*
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
/*
 *
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}
extern "C" void ln_raw_swd_reset(uint32_t pulses)
{
    rSWDIO->set(1);
    rSWDIO->output();
    for (int i = 0; i < pulses; i++)
        rSWCLK->pulseClock();
}
#include "../swd_template.h"
// EOF
