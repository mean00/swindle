#include "lnArduino.h"
#include "lnGPIO.h"
#include "lnBMP_pinout.h"

extern "C" void bmp_gpio_write(int pin, int value);
extern "C" int  bmp_gpio_read(int pin);
extern "C" void bmp_gpio_drive_state(int pin, int driven);

#define TRANSLATE(pin) lnPin xpin; xpin=_mapping[pin&7];
/**

*/
void defaultPinConf(int pin)
{
  TRANSLATE(pin);
  lnDigitalWrite(xpin,1);
  lnPinMode(xpin,lnOUTPUT);
}
/**
*/
void bmp_gpio_init()
{
  // all at vcc by default
  for(int i=0;i<BM_NB_PINS;i++)
    defaultPinConf(i);
  // Set reset to floating
  TRANSLATE(7);
  lnPinMode(xpin,lnINPUT_FLOATING);

}
/**
*/
void bmp_gpio_write(int pin, int value)
{
  TRANSLATE(pin);
  lnDigitalWrite(xpin,value);
}
/**
*/
int  bmp_gpio_read(int pin)
{
  TRANSLATE(pin);
  return lnDigitalRead(xpin);
}
/**

*/
void bmp_gpio_drive_state(int pin, int driven)
{
  TRANSLATE(pin);
  if(driven)
      lnPinMode(xpin,lnOUTPUT);
  else
      lnPinMode(xpin,lnINPUT_FLOATING);
}
/**
*/
extern "C" void platform_srst_set_val(bool assert)
{
  TRANSLATE(7);
  if(assert)
  {
    lnPinMode(xpin,lnOUTPUT);
    lnDigitalWrite(xpin,0);
  }
  else
    lnPinMode(xpin,lnINPUT_FLOATING);
}
/**
*/
extern "C" bool platform_srst_get_val(void)
{
  TRANSLATE(7);
  return lnDigitalRead(xpin);
}

// EOF
