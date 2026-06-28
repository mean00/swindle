/**
 * @file miniplatform.h
 * @brief Minimal platform abstraction for swindle targets.
 *
 * Provides logging, debug macros, and printf/stub overrides
 * pulled from the embedded_printf library.
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
#pragma once
#ifdef __cplusplus
extern "C"
{
#endif
    /** @brief Log a formatted message (platform-specific output). */
    extern void Logger(const char *fmt, ...);

    /** @brief Halt execution with an error code (non-returning). */
    extern void deadEnd(int err);
#define PLATFORM_HAS_DEBUG 1
#define ENABLE_DEBUG 1
#define ENABLE_RISCV 1

#undef PLATFORM_PRINTF
#define PLATFORM_PRINTF Logger
#include "printf.h"
#define snprintf snprintf_
// #define DEBUG(x, ...) do { ; } while (0)
#define DEBUG Logger
#ifdef __cplusplus
}
#endif
