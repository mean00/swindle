/**
 * @file bmp_reset_inv.cpp
 * @brief SwdReset implementation for inverted reset (active-high).
 *
 * The reset pin is configured as push-pull output:
 *   - Assert (on)  = drive high → target held in reset
 *   - De-assert     = drive low  → target released
 */
#include "bmp_pinout.h"
#include "esprit.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
// clang-format on

SwdReset::SwdReset(lnBMPPins no)
{
    _me = _mapping[no];
    setup();
}

void SwdReset::setup()
{
    _state = false;
    lnDigitalWrite(_me, 0);
    lnPinMode(_me, lnOUTPUT);
}

void SwdReset::on()
{
    _state = true;
    lnDigitalWrite(_me, 1);
}

void SwdReset::hiZ()
{
    lnDigitalWrite(_me, 0);
}

void SwdReset::off()
{
    _state = false;
    lnDigitalWrite(_me, 0);
}
// EOF
