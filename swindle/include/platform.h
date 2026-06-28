/**
 * @file platform.h
 * @brief Hosted-mode platform configuration for swindle (Linux/x86_64).
 *
 * Defines GPIO pin mapping, SWD signal macros, and printf/stub overrides
 * for the hosted (non-embedded) build target.
 *
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef __PLATFORM_H
#define __PLATFORM_H

#define NO_USB_PLEASE

#define SET_RUN_STATE(state)
#define SET_IDLE_STATE(state)
#define SET_ERROR_STATE(state)

#define ENABLE_RISCV 1

#undef sprintf
#undef snprintf
#undef vaprintf
#undef vasprintf
#include "embedded_printf/printf.h"
#include "timing.h"
#define vasprintf vasprintf_
#define snprintf snprintf_

#include "miniplatform.h"

#define fflush(x)                                                                                                      \
    {                                                                                                                  \
    }

#define TMS_SET_MODE()                                                                                                 \
    do                                                                                                                 \
    {                                                                                                                  \
    } while (0)

/** Virtual TMS pin (SWD/JTAG). */
#define TMS_PIN (8 + 0)
/** Virtual TDI pin (JTAG). */
#define TDI_PIN (8 + 1)
/** Virtual TDO pin (JTAG). */
#define TDO_PIN (8 + 2)
/** Virtual TCK pin (JTAG/SWD clock). */
#define TCK_PIN (8 + 3)
/** Virtual TRACESWO pin (SWO trace capture). */
#define TRACESWO_PIN (8 + 4)
/** Virtual SWDIO pin (SWD bidirectional data). */
#define SWDIO_PIN (8 + 5)
/** Virtual SWCLK pin (SWD clock). */
#define SWCLK_PIN (8 + 6)

#define SWCLK_PORT 0
#define SWDIO_PORT 0

#if 0
extern void bmp_gpio_write(int pin, int value);
extern int  bmp_gpio_read(int pin);
extern void bmp_gpio_drive_state(int pin, int driven);

#define gpio_set_val(port, pin, value) bmp_gpio_write(pin, value)
#define gpio_set(port, pin) bmp_gpio_write(pin, 1)
#define gpio_clear(port, pin) bmp_gpio_write(pin, 0)
#define gpio_get(port, pin) bmp_gpio_read(pin)
#define SWDIO_MODE_FLOAT() bmp_gpio_drive_state(SWDIO_PIN, false)
#define SWDIO_MODE_DRIVE() bmp_gpio_drive_state(SWDIO_PIN, true)

extern uint32_t swd_delay_cnt;
#endif
#endif
