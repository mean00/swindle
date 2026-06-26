/**
 * deps ../swindle/src/platform/rp2040/lnRP2040_pio
 * @file    swindle_rp2040_zero_w5500.cpp
 * @brief   RP2040-Zero + W5500 Ethernet debug probe.
 *          Defines board-specific pins and includes the common implementation.
 * Pin mapping (RP2040-Zero):
 *   W5500 CS   -> GPIO 5
 *   W5500 SCK  -> GPIO 2
 *   W5500 MOSI -> GPIO 3
 *   W5500 MISO -> GPIO 4
 *   W5500 RST  -> GPIO 6
 *   W5500 IRQ  -> GPIO 7
 * WS2812 LED on GPIO16 (RP2040-Zero).
 * @copyright (C) 2025
 * @license  See license file
 */

#include "lnGPIO.h"
#include "stdint.h"
#include "lowlevel_w5500.h"

// W5500 pin mapping (RP2040-Zero)
const lnW5500SPI w5500Pins = {
    .miso  = GPIO4,
    .mosi  = GPIO3,
    .clk   = GPIO2,
    .cs    = GPIO5,
    .reset = GPIO6,
    .intr  = GPIO7,
};

// MAC address (unique per device — customise per build)
const uint8_t mac[6] = {0x02, 0x00, 0x00, 0x00, 0x00, 0x02};

// WS2812 LED pin (GPIO16 for zero)
#define PIN_TO_USE GPIO16

#include "swindle_rp2040_w5500_common.cpp"