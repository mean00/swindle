
#include "esprit.h"
#include "lnGPIO.h"
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
