
#include "esprit.h"
#include "lnGPIO.h"
#include "lnBMP_reset.h"
/**
 */
class SwdWaitPin : public SwdPin
{
  public:
    SwdWaitPin(lnBMPPins no) : SwdPin(no)
    {
    }
    LN_ALWAYS_INLINE void clockOn()
    {
        on();
        if (_wait)
            swait();
    }
    LN_ALWAYS_INLINE void clockOff()
    {
        off();
        if (_wait)
            swait();
    }
    LN_ALWAYS_INLINE void wait()
    {
        if (_wait)
            swait();
    }
    LN_ALWAYS_INLINE void pulseClock()
    {
        clockOn();
        clockOff();
    }
};
