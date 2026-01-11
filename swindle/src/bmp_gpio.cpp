
#include "esprit.h"
//#include "lnBMP_pinout.h"
#include "lnGPIO.h"
extern "C"
{
#include "target/spi.h"
}
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
/*
void platform_nrst_set_val(bool assert)
{
    if (assert) // force reset to low
    {
        pReset.on();
    }
    else // release reset
    {
        pReset.off();
    }
}
bool platform_nrst_get_val(void)
{
   return pReset.state();
}
*/
void platform_target_clk_output_enable(bool enable)
{
    (void)enable;
}

bool platform_spi_init(const spi_bus_e bus)
{
    (void)bus;
    return false;
}

bool platform_spi_deinit(const spi_bus_e bus)
{
    (void)bus;
    return false;
}

bool platform_spi_chip_select(const uint8_t device_select)
{
    (void)device_select;
    return false;
}

uint8_t platform_spi_xfer(const spi_bus_e bus, const uint8_t value)
{
    (void)bus;
    return value;
}

// EOF
