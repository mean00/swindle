
#include "lnArduino.h"
#include "lnGPIO.h"

/**
 * @brief
 *
 */
class SwdReset
{
  public:
    SwdReset(lnBMPPins no)
    {
        _me = _mapping[no & 7];
        _state = false;
        // 1: Hi Z
        // 0: GND
        lnOpenDrainClose(_me, false);
        lnPinMode(_me, lnOUTPUT_OPEN_DRAIN, SWD_IO_SPEED);
    }

    void on()
    {
        _state = true;
        lnOpenDrainClose(_me, true);
    }
    void hiZ()
    {
        off();
    }
    void off()
    {
        _state = false;
        lnOpenDrainClose(_me, false);
    }
    bool state()
    {
        return _state;
    }

  protected:
    lnPin _me;
    bool _state;
};

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
