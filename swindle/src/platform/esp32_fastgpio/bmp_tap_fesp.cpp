/*
 *
 */
/**
 * @file bmp_tap_fesp.cpp
 * @brief ESP32 TAP initialisation and frequency control (fast GPIO)
 */

#include "bmp_pinout.h"
#include "stdint.h"
//
#include "hal/gpio_types.h"
//
#include "bmp_swdio_fesp.h"
#include "bmp_tap_fesp.h"
#include "esprit.h"
#include "lnCpuID.h"
#include "math.h"

#include "driver/dedic_gpio.h"
extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 10;          // around 1.3 Mbit/s
uint32_t swd_frequency = 1300 * 1000; // 1.3Mhz by default
SwdDirectionPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset *pReset;
/**
 * @brief Set the SWD wait-state (number of NOPs per bit-clock).
 * @param ws Wait-state count.
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    return;
}
/**
 * @brief Set the SWD clock frequency on ESP32 (dedicated GPIO).
 *
 * Converts Hz to wait-state count using 12.5 MHz calibration.
 * @param fq Desired SWCLK frequency in Hz.
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    uint32_t ws = 2;
    const float ratio = 12500000.f;
    if (fq < 10)
    {
        Logger("Invalid frequency\n");
        return;
    }
    if (fq > 10 * 1000 * 1000)
    {
        ws = 1;
    }
    else
    {
        ws = floor(0.49f + ratio / (float)fq);
    }
    // now reinvert it to update the actual fq
    float gf = ratio / (float)ws;
    swd_frequency = gf;
    bmp_set_wait_state_c((uint32_t)ws);
}
/**
 * @brief Get the current SWD wait-state count.
 * @return Number of NOPs per bit-clock.
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}
/**
 * @brief Initialise SWD GPIOs once at startup (dedicated GPIO bundle).
 *
 * Creates a two-pin dedicated-GPIO bundle for SWCLK and SWDIO,
 * enabling near-1-cycle bit-banging on ESP32.
 */
static dedic_gpio_bundle_handle_t bundle = NULL;
void bmp_gpio_init_once()
{
    Logger("Initializing IO with : \n");
    Logger("\t SWDIO : %d\n", _mapping[TSWDIO_PIN]);
    Logger("\t SWCLK : %d\n", _mapping[TSWDCK_PIN]);
    Logger("\t Reset : %d\n", _mapping[TRESET_PIN]);
    // create a fast gpio bundle for SWDCLK & SWDIO
    gpio_num_t io = (gpio_num_t)_mapping[TSWDIO_PIN];
    gpio_num_t ck = (gpio_num_t)_mapping[TSWDCK_PIN];
    //
    gpio_config_t io_conf = {};
    io_conf.pin_bit_mask = (1ULL << ck);
    io_conf.mode = GPIO_MODE_OUTPUT;
    io_conf.intr_type = GPIO_INTR_DISABLE; // nope
    gpio_config(&io_conf);

    io_conf.pin_bit_mask = (1ULL << io);
    io_conf.mode = GPIO_MODE_INPUT_OUTPUT; // Bidirectional
    io_conf.pull_up_en = GPIO_PULLUP_ENABLE;
    io_conf.intr_type = GPIO_INTR_DISABLE; // nope
    gpio_config(&io_conf);

    // 2. Create the Dedicated GPIO Bundle
    // Order in this array determines the bit position in the mask
    const int bundle_pins[] = {ck, io};
    dedic_gpio_bundle_config_t bundle_config = {.gpio_array = bundle_pins,
                                                .array_size = 2,
                                                .flags = {
                                                    .in_en = 1, // Required for reading bit 1
                                                    .in_invert = 0,
                                                    .out_en = 1,
                                                    .out_invert = 0,
                                                }};
    //
    // Bundle
    esp_err_t err = dedic_gpio_new_bundle(&bundle_config, &bundle);
    xAssert(err == ESP_OK);

    rSWDIO = new SwdDirectionPin(1);   // io is 1
    rSWCLK = new SwdWaitPin(0);        // ck is 0
    pReset = new SwdReset(TRESET_PIN); //
    pReset->setup();
    rSWDIO->output();
    pReset->off(); // hi-z by default
    rSWCLK->off();

    gmp_gpio_init_adc();
}
/**
 * @brief Begin an SWD session on ESP32 (dedicated GPIO).
 */
void bmp_io_begin_session()
{
    rSWDIO->on();
    rSWDIO->output();
    rSWCLK->clockOn();

    pReset->off(); // hi-z by default
}
/**
 * @brief End an SWD session on ESP32 (dedicated GPIO).
 */
void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    pReset->off(); // hi-z by default
}
/**
 * @brief Get the WS2812 data pin number for ESP32.
 * @return GPIO pin number.
 */
uint8_t ln_get_ws2812_pin()
{
    return LN_ESP_2812_PIN;
}
//
