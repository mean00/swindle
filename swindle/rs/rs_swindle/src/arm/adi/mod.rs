/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011-2015 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2018-2021 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2024 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the ARM Debug Interface (ADIv5/ADIv6/CoreSight) subsystem
 * of blackmagic/src/target/{adi,adiv5,adiv5_swd,adiv6}.c, made for the Swindle firmware. This
 * file is derived from those C sources and stays under the same license as the original
 * Black Magic code reproduced below.
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

//! ARM Debug Interface (ADIv5 / CoreSight) subsystem in pure Rust.

pub mod regs;
pub mod swd;
pub mod adiv5;
pub mod discovery;

pub use regs::*;
pub use swd::{HardwareSwd, SwdTransport};
pub use adiv5::{AccessPort, DebugPort, AdiError};
pub use discovery::*;
