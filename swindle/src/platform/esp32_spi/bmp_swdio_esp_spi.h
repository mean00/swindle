/**
 * @file      bmp_swdio_esp_spi.h
 * @brief     ESP32 SPI-based SWDIO pin abstraction.
 * @details   Inlines SWCLK/SWDIO pin control using direct GPIO register
 *            access (GPIO_OUT_W1TS / W1TC) for bit-bang operations.
 */
#pragma once
#include "lnBMP_reset.h"
extern "C"
{
#include "driver/gpio.h"
#include "soc/gpio_reg.h"
#include "soc/gpio_struct.h"
}
// clang-format on
//
//
/**/
