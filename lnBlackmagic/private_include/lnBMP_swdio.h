#pragma once

#define swait() {for(int lop = swd_delay_cnt; --lop > 0;) __asm__("nop");}

/**
*/
class SwdPin
{
public:
    SwdPin(lnBMPPins no, bool w=false) : _fast(_mapping[no&7])
    {
      _me=_mapping[no&7];
      _output=false;
      _wait=w;
      on();
      output();
    }
    void on()
    {
       _fast.on();
       if(_wait) swait();
    }
    void off()
    {
        _fast.off();
        if(_wait) swait();
    }
    void input()
    {
      lnPinMode(_me,lnINPUT_FLOATING);
    }
    void output()
    {
      lnPinMode(_me,lnOUTPUT);
    }
    void set(bool x)
    {
      if(x) on();
      else  off();
      //lnDigitalWrite(_me,x);
    }
    int read()
    {
        return lnDigitalRead(_me);
    }
protected:
  lnPin     _me;
  bool      _output;
  bool      _wait;
  lnFastIO  _fast;

};
