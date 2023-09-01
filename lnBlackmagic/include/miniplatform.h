/*
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
extern "C" {
#endif
extern void Logger(const char *fmt, ...);
extern void deadEnd(int err);
#define PLATFORM_HAS_DEBUG 1
#define ENABLE_DEBUG 1
#undef PLATFORM_PRINTF
#define PLATFORM_PRINTF Logger
// #define DEBUG(x, ...) do { ; } while (0)
#define DEBUG Logger
#ifdef __cplusplus
}
#endif
