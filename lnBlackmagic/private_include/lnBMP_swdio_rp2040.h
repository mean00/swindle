#pragma once

/**
 */
class SwdPin
{
  public:
    SwdPin(lnBMPPins no)
    {
        _me = _mapping[no & 7];
        _output = false;
        _wait = true;
        on();
        output();
    }

    void on()
    {
        lnDigitalWrite(_me, 1);
        //_fast.on();
    }
    void off()
    {
        lnDigitalWrite(_me, 0);
        //_fast.off();
    }
    void input()
    {
        lnPinMode(_me, lnINPUT_FLOATING);
    }
    void output()
    {
        lnPinMode(_me, lnOUTPUT, SWD_IO_SPEED); // 10 Mhz
    }
    void hiZ()
    {
        lnOpenDrainClose(_me, false);
        lnPinMode(_me, lnOUTPUT_OPEN_DRAIN, 1);
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
};
