#include "esprit.h"
#include "lnBMP_pinout.h"
#include "lnBMP_reset.h"
// clang-format on
//
SwdReset::SwdReset(lnBMPPins no)
{
    _me = _mapping[no];
    setup();
}
void SwdReset::setup()
{
    _state = false;
    lnDigitalWrite(_me, 0);
    lnPinMode(_me, lnOUTPUT, SWD_IO_SPEED);
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
