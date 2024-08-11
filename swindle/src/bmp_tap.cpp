#include "lnArduino.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
// clang-format on
uint32_t swd_delay_cnt = 4;
SwdPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset pReset(TRESET_PIN);
extern void bmp_extraSetWaitState();
#define INIT_ADC gmp_gpio_init_adc
void INIT_ADC();
extern void bmp_gpio_init_extra();
/**

*/
void bmp_gpio_init_once()
{
    bmp_gpio_init_extra();
    rSWDIO = new SwdPin(TSWDIO_PIN);
    rSWCLK = new SwdWaitPin(TSWDCK_PIN); // automatically add delay after toggle
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    rSWCLK->hiZ();
    rSWCLK->hiZ();
    pReset.hiZ(); // hi-z by default
    pReset.off(); // hi-z by default

    INIT_ADC();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
    rSWDIO->on();
    rSWDIO->output();
    rSWCLK->clockOn();
    rSWCLK->output();
    pReset.off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    rSWCLK->hiZ();
    rSWCLK->hiZ();
    pReset.off(); // hi-z by default
}
/**
 * @brief
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    bmp_extraSetWaitState();
}
/**
 * @brief
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}

/**
 */
extern "C" void platform_nrst_set_val(bool assert)
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
/**
 */
extern "C" bool platform_nrst_get_val(void)
{
    return pReset.state();
}

//
