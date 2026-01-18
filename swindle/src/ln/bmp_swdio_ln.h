#pragma once
/**
 */
#include "lnBMP_reset.h"

// clang-format on
static void __inline__ tapOutput();
static void __inline__ tapInput();
//
// #include "lnBMP_swdio_ln.h"
// #include "lnBMP_swdio_common.h"
//

// extern const lnPin _mapping[9];
class SwdPin
{
  public:
    SwdPin(lnBMPPins no) : _fast(_mapping[no])
    {
        _me = _mapping[no];
        _output = false;
        _wait = true;
        on();
        output();
    }

    LN_ALWAYS_INLINE void on()
    {
        _fast.on();
    }
    LN_ALWAYS_INLINE void off()
    {
        _fast.off();
    }
    LN_ALWAYS_INLINE void input()
    {
        lnPinMode(_me, lnINPUT_FLOATING);
    }
    LN_ALWAYS_INLINE void output()
    {
        lnPinMode(_me, lnOUTPUT, SWD_IO_SPEED); // 10 Mhz
    }
    void hiZ()
    {
        lnOpenDrainClose(_me, false);
        lnPinMode(_me, lnOUTPUT_OPEN_DRAIN, 1);
    }
    LN_ALWAYS_INLINE void set(bool x)
    {
        if (x)
            on();
        else
            off();
        // lnDigitalWrite(_me,x);
    }
    int read()
    {
        return lnDigitalRead(_me);
    }

  protected:
    lnPin _me;
    bool _output;
    bool _wait;
    lnFastIO _fast;
};
/**
 *
 *
 */
class SwdDirectionPin : public SwdPin
{
  public:
    SwdDirectionPin(lnBMPPins no) : SwdPin(no)
    {
        dir_output();
    }

    void dir_input()
    {
        lnPinMode(_me, lnINPUT_FLOATING);
        tapInput();
        currentDrive = false;
    }
    bool dir()
    {
        return currentDrive;
    }

    void dir_output()
    {
        lnPinMode(_me, lnOUTPUT, SWD_IO_SPEED); // 10 Mhz
        tapOutput();
        currentDrive = true;
    }
    bool currentDrive;
};
extern SwdPin *rDirection;

// clang-format on
void __inline__ tapOutput()
{
    rDirection->on();
}
void __inline__ tapInput()
{
    rDirection->off(); // off();
}
/**
 *
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
    LN_ALWAYS_INLINE void invClockOn()
    {
        swait();
        on();
    }
    LN_ALWAYS_INLINE void invClockOff()
    {
        swait();
        off();
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
    LN_ALWAYS_INLINE void invPulseClock()
    {
        swait();
        on();
        swait();
        off();
    }
};

/**/
