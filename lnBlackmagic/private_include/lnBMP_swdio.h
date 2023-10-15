#pragma once

#include "lnGPIO.h"
#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; --lop > 0;)                                                                      \
            __asm__("nop");                                                                                            \
    }

/**
 */
class SwdPin
{
  public:
    SwdPin(lnBMPPins no) : _fast(_mapping[no & 7])
    {
        _me = _mapping[no & 7];
        _output = false;
        _wait = true;
        on();
        output();
    }

    void on()
    {
        _fast.on();
    }
    void off()
    {
        _fast.off();
    }
    void input()
    {
        lnPinMode(_me, lnINPUT_FLOATING);
    }
    void output()
    {
        lnPinMode(_me, lnOUTPUT);
    }
    void set(bool x)
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
        lnDigitalWrite(_me,LN_GPIO_OUTPUT_OD_HIZ); // hi Z
        lnPinMode(_me, lnOUTPUT_OPEN_DRAIN);
    }

    void on()
    {
        _state = true;
       lnDigitalWrite(_me,LN_GPIO_OUTPUT_OD_GND);
    }
    void off()
    {
        _state = false;
       lnDigitalWrite(_me,LN_GPIO_OUTPUT_OD_HIZ);
    }   
    bool state() { return _state;}
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
    void clockOn()
    {
        _fast.on();
        if (_wait)
            swait();
    }
    void clockOff()
    {
        _fast.off();
        if (_wait)
            swait();
    }
};
