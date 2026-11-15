/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2025 1BitSquared <info@1bitsquared.com>
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

/*
 * Flash programming stub for the STM32F1-style FPEC.
 *
 * The probe loads this next to a chunk of data in the target's SRAM and calls it, so the
 * chunk crosses the debug link once, as a plain memory write, and the target programs it
 * itself at core speed instead of one bus-stalled halfword per debug transaction.
 *
 * Built twice by `fpec_mkstub.sh`, differing only in the size of one program step:
 *  - FPEC_WRITE_WIDTH == 2 for the STM32F0/F1/F3 parts (16-bit steps),
 *  - FPEC_WRITE_WIDTH == 4 for the GD32/AT32 clones that accept 32-bit steps.
 *
 * Arguments (Cortex-M ABI): r0 = flash destination, r1 = SRAM source, r2 = byte count,
 * r3 = address of the FPEC status register of the bank being programmed (the second bank
 * of the dual-bank parts has its own copy). Publishes the common stub exit code: `bkpt 1`
 * once the whole chunk is programmed, `bkpt 0` when the controller reports an error.
 */

#include <stdint.h>
#include "stub.h"

#ifndef FPEC_WRITE_WIDTH
#error "fpec_write.c needs FPEC_WRITE_WIDTH (2 or 4)"
#endif

#if FPEC_WRITE_WIDTH == 4
typedef uint32_t fpec_unit_t;
#elif FPEC_WRITE_WIDTH == 2
typedef uint16_t fpec_unit_t;
#else
#error "fpec_write.c only supports 16- and 32-bit program steps"
#endif

/* `SR` bits of the FPEC, see `registers.rs`. */
#define FPEC_STATUS_BSY (1U << 0U)
#define FPEC_STATUS_ERROR_MASK 0x14U

/*
 * Steps programmed back to back before the status register is looked at again. The
 * controller stalls the bus on a store while it is busy, so the store itself does the
 * waiting -- that is exactly what the probe-side `write_aligned()` path relies on. A
 * read of `SR` per step only costs time: the probe resets the target before it programs,
 * so the core is usually still on the 8 MHz clock it comes out of reset with, where one
 * read is worth tens of microseconds and used to halve the throughput of this stub.
 * Checking every 32 steps keeps the polls amortised and still bails out early on an error.
 */
#define FPEC_STEPS_PER_STATUS_CHECK 32U

void __attribute__((naked)) __attribute__((noreturn)) fpec_write_stub(
	const fpec_unit_t *dest, const fpec_unit_t *src, uint32_t len, const volatile uint32_t *status)
{
	while (len >= FPEC_WRITE_WIDTH) {
		for (uint32_t step = 0; step < FPEC_STEPS_PER_STATUS_CHECK && len >= FPEC_WRITE_WIDTH;
			 ++step) {
			*(volatile fpec_unit_t *)dest = *src;
			++dest;
			++src;
			len -= FPEC_WRITE_WIDTH;
		}
		/* Drop out as soon as the controller reports an error: the remaining steps would
		 * only fail again. */
		while (*status & FPEC_STATUS_BSY)
			continue;
		if (*status & FPEC_STATUS_ERROR_MASK)
			stub_exit(0);
	}
	stub_exit(1);
	for (;;)
		continue;
}

// EOF
