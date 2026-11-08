
/**
 * @file bmp_gpio.cpp
 * @brief GPIO mode setup and SWDIO/SWCLK pin initialisation
 */

#include "esprit.h"
#include "lnGPIO.h"
extern "C"
{
#include "platform_support.h"
}
extern "C" void bmp_gpio_write(int pin, int value);
extern "C" int bmp_gpio_read(int pin);
extern "C" void bmp_gpio_drive_state(int pin, int driven);

extern "C" void platform_nrst_set_val(bool assert);
extern "C" void platform_target_clk_output_enable(bool enable);
extern "C" bool platform_nrst_get_val(void);

/**
 * @brief Write to a BMP GPIO pin.
 * @param pin   Pin number.
 * @param value 0 or 1.
 * @note Always asserts — not implemented; platform code overrides.
 */
void bmp_gpio_write(int pin, int value)
{
    xAssert(0);
}
/**
 * @brief Read a BMP GPIO pin.
 * @param pin Pin number.
 * @return 0
 * @note Always asserts — not implemented; platform code overrides.
 */
int bmp_gpio_read(int pin)
{
    xAssert(0);
    return 0;
}
void platform_target_clk_output_enable(bool enable)
{
    (void)enable;
}

// EOF
