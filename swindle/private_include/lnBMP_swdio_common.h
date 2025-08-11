
#include "esprit.h"
#include "lnGPIO.h"

/**
 * @brief
 *
 */
// In case of inverted reset, writting 1 pulls reset to zero
// It is a digital pin
#ifdef SWINDLE_INVERTED_RESET
class SwdReset
{
  public:
    SwdReset(lnBMPPins no)
    {
        _me = _mapping[no];
        _state = false;
        // 1: Hi Z
        // 0: GND
        lnDigitalWrite(_me, 0);
        lnPinMode(_me, lnOUTPUT);
    }

    void on()
    {
        _state = true;
        lnDigitalWrite(_me, 1);
    }
    void hiZ()
    {
        lnDigitalWrite(_me, 0);
    }
    void off()
    {
        _state = false;
        lnDigitalWrite(_me, 0);
    }
    bool state()
    {
        return _state;
    }

  protected:
    lnPin _me;
    bool _state;
};
#else
// It is slightly different for non inverted reset
// in that case the gpio is an open drain
class SwdReset
{
  public:
    SwdReset(lnBMPPins no)
    {
        _me = _mapping[no];
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
#endif
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
