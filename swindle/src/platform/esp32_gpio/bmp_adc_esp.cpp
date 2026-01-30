
#include "esprit.h"
#include "lnADC.h"
#include "bmp_pinout.h"
static lnSimpleADC *adc = NULL;
/**
 * @brief
 *
 */
void gmp_gpio_init_adc()
{
}

/**
 * @brief
 *
 */
extern "C" float bmp_get_target_voltage_c()
{
    return 1.0;
}
// EOF
