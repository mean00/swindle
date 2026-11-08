
/**
 * @file bmp_adc_ln.cpp
 * @brief GD32/WCH ADC driver for target voltage measurement
 */

#include "bmp_pinout.h"
#include "esprit.h"
#include "lnADC.h"

/**
 * @brief Initialise the ADC peripheral for target voltage sensing.
 */
void gmp_gpio_init_adc()
{
    lnPinMode(PIN_ADC_NRESET_DIV_BY_TWO, lnADC_MODE);
    lnPeripherals::enable(Peripherals::pADC0);
}
// EOF
