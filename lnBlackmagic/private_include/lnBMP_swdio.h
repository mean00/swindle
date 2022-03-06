#pragma once

#define swait() {for(int lop = swd_delay_cnt; --lop > 0;) __asm__("nop");}

/**
*/
class SwdPin
{
public:
    SwdPin(lnBMPPins no, bool w=false)
    {
      _me=_mapping[no&7];
      _output=false;
      _wait=w;
      on();
      output();
    }
    void on()
    {
         lnDigitalWrite(_me,true);
         if(_wait) swait();
    }
    void off()
    {
          lnDigitalWrite(_me,false);
          if(_wait) swait();
    }
    void input()
    {
      lnDigitalWrite(pinDir,false);
      lnPinMode(_me,lnINPUT_FLOATING);
    }
    void output()
    {
      lnPinMode(_me,lnOUTPUT);
      lnDigitalWrite(pinDir,true);
    }
    void set(bool x)
    {
        lnDigitalWrite(_me,x);
    }
    int read()
    {
        return lnDigitalRead(_me);
    }
protected:
  lnPin     _me;
  bool      _output;
  bool      _wait;

};
