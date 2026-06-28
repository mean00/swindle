/**
 * @file bmp_reset.cpp
 * @brief SwdReset implementation for non-inverted, open-drain reset.
 *
 * The reset pin is configured as open-drain:
 *   - Assert (on)  = drive low  → target held in reset
 *   - De-assert     = Hi-Z      → pull-up releases target
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
    // 1: Hi Z, 0: GND
    lnOpenDrainClose(_me, false);
    lnPinMode(_me, lnOUTPUT_OPEN_DRAIN);
}

void SwdReset::on()
{
    _state = true;
    lnOpenDrainClose(_me, true);
}

void SwdReset::hiZ()
{
    off();
}

void SwdReset::off()
{
    _state = false;
    lnOpenDrainClose(_me, false);
}
//
