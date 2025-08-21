#include "esprit.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
// clang-format on
//
//
// It is slightly different for non inverted reset
// in that case the gpio is an open drain
SwdReset::SwdReset(lnBMPPins no)
{
    _me = _mapping[no];
    setup();
}
void SwdReset::setup()
{
    _state = false;
    // 1: Hi Z
    // 0: GND
    lnOpenDrainClose(_me, false);
    lnPinMode(_me, lnOUTPUT_OPEN_DRAIN, SWD_IO_SPEED);
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
