/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the per-family driver split of blackmagic/src/target/stm32f1.c and
 * stm32f4.c, made for the Swindle firmware. This file is derived from those C sources
 * and stays under the same license as the original Black Magic code reproduced below.
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

//! Per-family drivers. Each module owns the probe and the part table of one STM32 /
//! clone family; `flash.rs` and `flash4.rs` hold the flash-controller code the families
//! share.

pub mod ch32f1;
pub mod f1;
pub mod f1_clones;
