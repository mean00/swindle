/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2014 Mike Walters <mike@flomp.net>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Written by Mike Walters <mike@flomp.net>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of `blackmagic/src/target/nrf51.c`, `nrf54l.c` and `nrf91.c` (the Nordic
 * drivers), made for the Swindle firmware. This file is derived from that C source and
 * stays under the same license as the original Black Magic code reproduced below.
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

#![no_std]

//! Pure Rust drivers for Nordic Semiconductor targets:
//! - nRF51 / nRF52 series (`Nrf51Driver`)
//! - nRF54L series (`Nrf54lDriver`)
//! - nRF91 series (`Nrf91Driver`)
//! - CTRL-AP recovery access port drivers (`ctrl_ap`)

pub mod ctrl_ap;
pub mod mon_commands;
pub mod nrf51;
pub mod nrf54l;
pub mod nrf91;
pub mod registers;

pub use ctrl_ap::{nrf51_ctrl_ap_probe, nrf54l_ctrl_ap_probe};
pub use nrf51::Nrf51Driver;
pub use nrf54l::Nrf54lDriver;
pub use nrf91::Nrf91Driver;
