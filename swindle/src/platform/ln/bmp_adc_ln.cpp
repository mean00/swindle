
/**
 * @file bmp_adc_ln.cpp
 * @brief GD32/WCH ADC driver for target voltage measurement
 */

#include "bmp_pinout.h"
#include "esprit.h"
#include "lnADC.h"
static lnSimpleADC *adc = NULL;
/**
 * @brief Initialise the ADC peripheral for target voltage sensing.
 */
void gmp_gpio_init_adc()
{
    lnPinMode(PIN_ADC_NRESET_DIV_BY_TWO, lnADC_MODE);
    lnPeripherals::enable(Peripherals::pADC0);
    adc = new lnSimpleADC(0, PIN_ADC_NRESET_DIV_BY_TWO);
    adc->setSmpt(LN_ADC_SMPT_239_5);
}

/**
 * @brief Measure target MCU voltage through the ADC.
 *
 * Averages 16 ADC samples and converts to millivolts.
 * @return Target voltage in millivolts, or 0.0 if Vref is invalid.
 */
extern "C" float bmp_get_target_voltage_c()
{
    float vcc = 3300.; // lnBaseAdc::getVcc();
    if (vcc < 2600)
    {
        Logger("Invalid ADC Vref\n");
        return 0.0;
    }
    int sample = 0;
    for (int i = 0; i < 16; i++)
    {
        sample += adc->simpleRead();
    }
    sample /= 16;

    vcc = (float)sample * vcc * (float)PIN_ADC_NRESET_MULTIPLIER; // need to multiply by PIN_ADC_NRESET_MULTIPLIER
    vcc = vcc / 4095000.f;
    return vcc;
}
// EOF
