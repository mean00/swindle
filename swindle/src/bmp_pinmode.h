/**
 * @file bmp_pinmode.h
 * @brief Pin mode selection for SWD/JTAG GPIO reconfiguration.
 *
 * Derived from the Black Magic Debug project but simplified to focus on
 * the subset actually used (SWD, RVSWD, GPIO).
 */
#pragma once

/** @brief Available pin operating modes. */
enum bmp_pin_mode
{
    BMP_PINMODE_NONE,      /**< Pin not configured (default / safe state). */
    BMP_PINMODE_GPIO,      /**< Standard digital GPIO mode. */
    BMP_PINMODE_SWD,       /**< ARM SWD protocol (SWDIO + SWCLK). */
    BMP_PINMODE_RVSWD,     /**< RISC-V RVSWD protocol (variant of SWD). */
    BMP_PINMODE_RVSWD_RAW, /**< RISC-V RVSWD RAW (no start/stop bits). */
};

/** @brief Reconfigure debug pins to the specified operating mode. */
extern void bmp_gpio_pinmode(bmp_pin_mode pioMode);
