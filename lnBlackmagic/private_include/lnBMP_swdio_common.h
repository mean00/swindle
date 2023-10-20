
#include "lnGPIO.h"

#define SWD_IO_SPEED 10 // 10mhz is more than enough (?)

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; --lop > 0;)                                                                      \
            __asm__("nop");                                                                                            \
    }


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
        lnPinMode(_me, lnOUTPUT_OPEN_DRAIN,SWD_IO_SPEED);
    }

    void on()
    {
        _state = true;
       lnDigitalWrite(_me,LN_GPIO_OUTPUT_OD_GND);
    }
    void hiZ()
    {
        off();
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
        on();
        if (_wait)
            swait();
    }
    void clockOff()
    {
        off();
        if (_wait)
            swait();
    }
};