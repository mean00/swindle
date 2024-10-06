/*
 * Reconfingiure the pins to swich to the given mode
  This code is derived from the blackmagic one but has been modified
  to aim at simplicity at the expense of performances (does not matter much though)
  (The compiler may mitigate that by inlining)

 */
#pragma once
enum bmp_pin_mode
{
    BMP_PINMODE_NONE,
    BMP_PINMODE_GPIO,
    BMP_PINMODE_SWD,
    BMP_PINMODE_RVSWD,
};
extern void bmp_gpio_pinmode(bmp_pin_mode pioMode);
