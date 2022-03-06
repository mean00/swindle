
#include "lnArduino.h"
#include "lnGPIO.h"
#include "lnBMP_pinout.h"

extern "C" void bmp_gpio_write(int pin, int value);
extern "C" int  bmp_gpio_read(int pin);
extern "C" void bmp_gpio_drive_state(int pin, int driven);

/**
*/
void bmp_gpio_write(int pin, int value)
{
  xAssert(0);
}
/**
*/
int  bmp_gpio_read(int pin)
{
  xAssert(0);
  return 0;
}

// EOF
