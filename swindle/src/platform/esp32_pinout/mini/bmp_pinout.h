#pragma once
#include "lnGPIO_pins.h"
// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[9] = {
    GPIO0, // 0 TMS_PIN
    GPIO0, // 1 TDI_PIN
    GPIO0, // 2 TDO_PIN
    GPIO0, // 3 TCK_PIN
    GPIO0, // 4 TRACESWO_PIN

    GPIO7, // 5 SWDIO_PIN
    GPIO8, // 6 SWCLK_PIN

    GPIO9,  // 7 RST
    GPIO10, // 8 direction
};

#define PIN_ADC_NRESET_DIV_BY_TWO PA3 // this pins is connected to NRST/2
#define LN_ESP_2812_PIN 21
