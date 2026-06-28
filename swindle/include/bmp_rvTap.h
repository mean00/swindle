/**
 * @file bmp_rvTap.h
 * @brief RISC-V DMI debug transport over bit-banged GPIO (platform-agnostic)
 */

/*
  lnBMP: Gpio driver for Rvswd
  This code is derived from the blackmagic one but has been modified
  to aim at simplicity at the expense of performances (does not matter much though)
  (The compiler may mitigate that by inlining)

Original license header

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


 This file implements the WCH DMI/Serial interface.

CLK **HIGH**
        IO --___  START
        IO __---  STOP

IO is sampled when clock goes ___---

 */
#include "esprit.h"
#pragma once

/**
 * @brief Write a 32-bit value to a RISC-V Debug Module register via the DMI interface.
 * @param adr  Address of the DM register to write.
 * @param val  32-bit value to write.
 * @return true on success, false on DMI bus error.
 */
bool rv_dm_write(uint32_t adr, uint32_t val);
/**
 * @brief Read a 32-bit value from a RISC-V Debug Module register via the DMI interface.
 * @param adr    Address of the DM register to read.
 * @param output Pointer to a uint32_t receiving the read value.
 * @return true on success, false on DMI bus error.
 */
bool rv_dm_read(uint32_t adr, uint32_t *output);

// bool rv_dm_reset();
//  EOF
