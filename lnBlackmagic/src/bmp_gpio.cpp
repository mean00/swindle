#include "lnArduino.h"
#include "lnGPIO.h"


extern "C" void bmp_gpio_write(int pin, int value);
extern "C" int  bmp_gpio_read(int pin);
extern "C" void bmp_gpio_drive_state(int pin, int driven);

#define BM_NB_PINS 7
// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[8]=
{
    PA10, // 0 TMS_PIN
    PA10, // 1 TDI_PIN
    PA10, // 2 TDO_PIN
    PA10, // 3 TCK_PIN
    PA10, // 4 TRACESWO_PIN
    PA10, // 5 SWDIO_PIN
    PA10, // 6 SWCLK_PIN
    (lnPin)-1
};

#define TRANSLATE(pin) lnPin xpin; xpin=_mapping[pin&7];
/**

*/
// https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/gpio.html
void defaultPinConf(int pin)
{
  TRANSLATE(pin);
  lnDigitalWrite(xpin,1);
  lnPinMode(xpin,lnOUTPUT);
}
void bmp_gpio_init()
{
  // all at vcc by default
  for(int i=0;i<BM_NB_PINS;i++)
    defaultPinConf(i);
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
// EOF
