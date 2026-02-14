/*
 *
 */
#include "esprit.h"
#include "bmp_pinout.h"
#include "bmp_swdio_esp.h"
#include "lnCpuID.h"
#include "math.h"
#include "bmp_tap_esp.h"

#include <driver/dedic_gpio.h>
extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 1;
uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
SwdDirectionPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset *pReset;
/**
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    return;
}
/**
 * @brief
 *
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    float alpha = 0, beta = 0;
    if (fq < 10)
    {
        Logger("Invalid frequency\n");
        return;
    }
    alpha = 2.3 * 1000000.;
    beta = 0.0;
    // convert fq to wait state
    float wf = ((alpha) / (float)fq) - beta;
    if (wf < 0.0)
        wf = 0.;

    // get wf which is the wait state as rounded int
    wf = floor(wf + 0.49);
    // now reinvert it to update the actual fq
    float gf = (alpha) / (wf + beta);
    swd_frequency = gf;
    bmp_set_wait_state_c((uint32_t)wf);
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
dedic_gpio_bundle_handle_t bundle = NULL;
void bmp_gpio_init_once()
{
    // configure SWDIO pin as pullup
    gpio_num_t io = (gpio_num_t)_mapping[TSWDIO_PIN];
    gpio_reset_pin(io);
    // The SWDIO is pullup + od
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << io),
        .mode = GPIO_MODE_INPUT_OUTPUT_OD,     // Input + Output Open-Drain
        .pull_up_en = GPIO_PULLUP_ENABLE,      // Internal pull-up to keep it High by default
        .pull_down_en = GPIO_PULLDOWN_DISABLE, // nope
        .intr_type = GPIO_INTR_DISABLE,        // nope
    };
    gpio_config(&io_conf);
    // clk
    gpio_num_t clk = (gpio_num_t)_mapping[TSWDCK_PIN];
    gpio_reset_pin(clk);
    // The SWDIO is pullup + od
    gpio_config_t clk_conf = {
        .pin_bit_mask = (1ULL << clk),
        .mode = GPIO_MODE_OUTPUT_OD,           // Input + Output Open-Drain
        .pull_up_en = GPIO_PULLUP_ENABLE,      // Internal pull-up to keep it High by default
        .pull_down_en = GPIO_PULLDOWN_DISABLE, // nope
        .intr_type = GPIO_INTR_DISABLE,        // nope
    };
    gpio_config(&clk_conf);

    // Prepare fast GPIO bundle for SWDCLK  & SWDIO
    dedic_gpio_bundle_config_t bundle_cfg = {
        .gpio_array = (int[]){_mapping[TSWDCK_PIN], _mapping[TSWDIO_PIN]},
        .array_size = 2,
        .flags = {
            .in_en = 1,      // Enable input for SWDIO
            .in_invert = 0,  // Enable input for SWDIO
            .out_en = 1,     // Enable output for both (SWDIO will toggle direction)
            .out_invert = 0, // Enable output for both (SWDIO will toggle direction)
        }};

    // Bundle
    esp_err_t err = dedic_gpio_new_bundle(&bundle_cfg, &bundle);
    xAssert(err == ESP_OK);

    rSWDIO = new SwdDirectionPin(1 << 1); //
    rSWCLK = new SwdWaitPin(1 << 0);      //
    pReset = new SwdReset(TRESET_PIN);    //
    pReset->setup();
    rSWDIO->output();
    pReset->off(); // hi-z by default
    rSWCLK->off();

    gmp_gpio_init_adc();
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

    pReset->off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    pReset->off(); // hi-z by default
}
//
