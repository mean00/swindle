/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2012-2020  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>,
 * Copyright (C) 2022-2023 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 * Koen De Vleeschauwer and Uwe Bonnes
 *
 * Rust port of the module split of `blackmagic/src/target/cortexm.c`, made for the Swindle
 * firmware. This file is derived from that C source and stays under the same license as
 * the original Black Magic code reproduced below.
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

/// Cortex-M stub runner (native Rust port of the C `cortexm_run_stub()`).
pub mod stub;

/// Generic Cortex debug layer (native port of `blackmagic/src/target/cortex.c`): the
/// access-port link, the debug-window accessors and the CPUID decode.
pub mod cortex;

/// Cortex-M debug layer (native port of `blackmagic/src/target/cortexm.c`): probe, attach,
/// detach, `vector_catch`, and `CortexMTargetDriver`.
pub mod cortexm;

/// Cortex-M memory access and D-cache maintenance.
pub mod cortexm_memaccess;

/// Cortex-M banked register file transfers (`DCRSR` / `DCRDR`).
pub mod cortexm_regs;

/// Cortex-M hardware breakpoints (FPB) and watchpoints (DWT).
pub mod cortexm_breakpoint;

/// Cortex-M execution control: reset, halt request/poll/resume, fault unwinding, semihosting,
/// and the flash stub runner.
pub mod cortexm_halt;

/// Native ARM Debug Interface (ADIv5, SWD, and CoreSight) subsystem.
pub mod adi;

/// The `CORTEXM_*` register map of this layer: addresses, bit fields and register-file sizes.
pub mod cortexm_registers;

/// The C seam: safe wrappers for ADIv5 operations.
pub mod c_seam;

/// Dispatch of the ARM target driver that is currently probed: the C `target_s`
/// configuration and the flash/attach/CRC callbacks around it.
pub mod driver_registry;

pub use stub::*;
pub use driver_registry::*;
pub use cortexm::CortexMTargetDriver;
