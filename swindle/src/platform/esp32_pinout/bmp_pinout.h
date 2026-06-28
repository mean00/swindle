/**
 * @file bmp_pinout.h
 * @brief ESP32 pin mapping lookup function
 */

#pragma once
#include "lnGPIO.h"
//
#include "lnGPIO_pins.h"

#ifdef LN_ESP_MINI
#include "mini/bmp_pinout.h"
#else
#include "wroom/bmp_pinout.h"
#endif

#define PIN_ADC_NRESET_MULTIPLIER 1. // 2.0 if divided by 2 , 1.0 if not divided

#define LN_USB_INSTANCE 1
#define LN_SERIAL_INSTANCE 2
#define LN_LOGGER_INSTANCE 2

extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }
