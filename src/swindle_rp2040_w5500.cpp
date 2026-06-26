/**
 * deps ../private_include/w5500_gdb_task,../swindle/src/platform/rp2040/lnRP2040_pio
 * @file    swindle_rp2040_w5500.cpp
 * @brief   RP2040 + W5500 Ethernet debug probe (full-size RP2040 carrier).
 *          Defines board-specific pins and includes the common implementation.
 * Pin mapping (full-size RP2040, e.g. Pico or carrier board):
 *   W5500 CS   -> GPIO 17
 *   W5500 SCK  -> GPIO 18
 *   W5500 MOSI -> GPIO 19
 *   W5500 MISO -> GPIO 16
 *   W5500 RST  -> GPIO 21
 *   W5500 IRQ  -> GPIO 20
 * WS2812 LED on GPIO23 (full-size RP2040).
 * @copyright (C) 2025
 * @license  See license file
 */

#include "lnGPIO.h"
#include "stdint.h"
#include "lowlevel_w5500.h"
//

#define PIN_TO_USE GPIO23
// W5500 pin mapping (full-size RP2040)
const lnW5500SPI w5500Pins = {
    .miso = GPIO16,
    .mosi = GPIO19,
    .clk = GPIO18,
    .cs = GPIO17,
    .reset = GPIO21,
    .intr = GPIO20,
};

// MAC address (unique per device — customise per build)
const uint8_t mac[6] = {0x02, 0x00, 0x00, 0x00, 0x00, 0x01};

#include "swindle_rp2040_w5500_common.cpp"
