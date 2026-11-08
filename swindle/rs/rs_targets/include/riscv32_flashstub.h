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

#ifndef TARGET_RISCV_FLASHSTUB_STUB_H
#define TARGET_RISCV_FLASHSTUB_STUB_H

/*
 * Stub exit helper shared by every RISC-V flashstub source in the Rust tree:
 * `rs_ch32v00x/flashstub`, `rs_ch32v123xx/flashstub` and `rs_rp2350_common/flashstub`
 * (which also has the RP2350 C twin). It lives here, next to the crates, so a stub build
 * never has to reach into the blackmagic tree.
 *
 * Same convention as the Cortex-M `stub_exit()`: 1 (= STUB_EXIT_OK) means the stub did its
 * work, 0 (= STUB_EXIT_FAIL) means it failed. The stub runner compares a0 against 1, so
 * never publish anything else as a success code.
 */
static inline void __attribute__((always_inline)) riscv_stub_exit(const int code)
{
    __asm__("mv a0, %0\n"
            "ebreak" 
            :
            :"r"(code)
            : "a0");
}

#endif
