/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2015  Black Sphere Technologies Ltd.
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

#ifndef TARGET_CORTEXM_FLASHSTUB_H
#define TARGET_CORTEXM_FLASHSTUB_H

/*
 * Stub exit helper shared by Cortex-M flashstub sources in the Rust tree:
 * `rs_stm32f1/src/flashstub` and `rs_rp_common/src/flashstub`.
 *
 * Same convention as the RISC-V `riscv_stub_exit()` (`riscv32_flashstub.h`):
 * 1 (= STUB_EXIT_OK) means the stub did its work, 0 (= STUB_EXIT_FAIL) means it
 * failed. The stub runner compares the bkpt immediate against 1, so never
 * publish anything else as a success code.
 */
static inline void __attribute__((always_inline)) stub_exit(const int code)
{
    __asm__("bkpt %0" ::"i"(code));
}

#endif /* TARGET_CORTEXM_FLASHSTUB_H */
