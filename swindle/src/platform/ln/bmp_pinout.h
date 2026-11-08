/**
 * @file bmp_pinout.h
 * @brief LN (GD32F303 / CH32V307) debug pin mapping.
 *
 * The pins are the probe board table's, not a header of their own: the selected
 * board file (swindle/rs/rs_swindle_board/src/bluepill/ln_48pins.rs or ln_64pins.rs) is
 * rendered as bmp_pinout_board.h by swindle/board.cmake, which defines
 * _mapping[], PIN_ADC_NRESET_DIV_BY_TWO / PIN_ADC_NRESET_MULTIPLIER and the
 * LN_*_INSTANCE values. Change the board file, not this header.
 */

#pragma once
#define SWD_IO_SPEED 10 // programmed max speed of the gpio, 10 Mhz is  more than enough
#include "lnBMP_pins.h"
//
#include "lnGPIO.h"
//
#include "bmp_pinout_board.h"

extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }

// EOF
