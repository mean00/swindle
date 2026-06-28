/**
 * @file bmp_pinout.h
 * @brief ESP32 WROOM pin mapping table
 */

#pragma once

// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[9] = {
    GPIO0, // 0 TMS_PIN
    GPIO0, // 1 TDI_PIN
    GPIO0, // 2 TDO_PIN
    GPIO0, // 3 TCK_PIN
    GPIO0, // 4 TRACESWO_PIN

    GPIO18, // 5 SWDIO_PIN
    GPIO17, // 6 SWCLK_PIN

    GPIO2, // 7 RST
    GPIO3, // 8 direction
};

#define PIN_ADC_NRESET_DIV_BY_TWO 4 // GPIO4
#define LN_ESP_2812_PIN 48
