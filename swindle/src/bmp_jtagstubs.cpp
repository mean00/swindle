/**
 * @file bmp_jtagstubs.cpp
 * @brief JTAG stubs — disable JTAG to save flash space.
 *
 * Provides empty implementations of the JTAG-DP interface so that
 * the linker can discard the real JTAG code. Only SWD/RVSWD is used.
 *
 * Original license header:

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
#include "esprit.h"
// #include "lnBMP_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "exception.h"
#include "general.h"
#include "jtagtap.h"
#include "timing.h"

    /** @brief JTAG procedure table — all NULL (JTAG disabled). */
    extern jtag_proc_s jtag_proc = {NULL, NULL, NULL, NULL, NULL};

    /** @brief JTAG tap init stub — no-op when JTAG is disabled. */
    void jtagtap_init()
    {
        return;
    }

    /** @brief JTAG scan stub. @return false (no devices found). */
    bool jtag_scan()
    {
        return false;
    }

    /** @brief JTAG-DP abort stub — no-op. */
    void adiv5_jtagdp_abort(adiv5_debug_port_s *dp, uint32_t abort)
    {
    }

    /** @brief JTAG-DP read stub. @return 0. */
    uint32_t fw_adiv5_jtagdp_read(adiv5_debug_port_s *dp, uint16_t addr)
    {
        return 0;
    }

    /** @brief JTAG-DP low-access stub — raises EXCEPTION_ERROR.
     * @return 0 (unreachable). */
    uint32_t fw_adiv5_jtagdp_low_access(adiv5_debug_port_s *dp, uint8_t RnW, uint16_t addr, uint32_t value)
    {
        raise_exception(EXCEPTION_ERROR, "JTAG-DP disabled");
        return 0;
    }

    /** @brief JTAG add-device stub — no-op. */
    void jtag_add_device(const uint32_t dev_index, const jtag_dev_s *jtag_dev)
    {
    }
}

/** @brief Debug test stub — empty. */
extern "C" void resetTest2()
{
}
