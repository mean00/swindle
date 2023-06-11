
#include "lnArduino.h"
#include "lnBMP_pinout.h"
#include "lnGPIO.h"

extern "C" void bmp_gpio_write(int pin, int value);
extern "C" int bmp_gpio_read(int pin);
extern "C" void bmp_gpio_drive_state(int pin, int driven);

extern "C" void platform_nrst_set_val(bool assert);
extern "C" void platform_target_clk_output_enable(bool enable);
extern "C" bool platform_nrst_get_val(void);

/**
 */
void bmp_gpio_write(int pin, int value)
{
    xAssert(0);
}
/**
 */
int bmp_gpio_read(int pin)
{
    xAssert(0);
    return 0;
}

void platform_nrst_set_val(bool assert)
{
    lnPin pin = _mapping[TRESET_PIN];
    if (assert)
    {
        lnPinMode(pin, lnOUTPUT);
        lnDigitalWrite(pin, 0);
    }
    else
    {
        lnPinMode(pin, lnINPUT_FLOATING);
    }
}
bool platform_nrst_get_val(void)
{

    return lnDigitalRead(_mapping[TRESET_PIN]);
}

void platform_target_clk_output_enable(bool enable)
{
    (void)enable;
}

// EOF
