
#include "esprit.h"
#include "lnGPIO.h"
#include "lnBMP_pins.h"
#pragma once
class SwdReset
{
  public:
    SwdReset(lnBMPPins no);
    void on();
    void setup();
    void hiZ();
    void off();
    bool state()
    {
        return _state;
    }

  protected:
    lnPin _me;
    bool _state;
};
