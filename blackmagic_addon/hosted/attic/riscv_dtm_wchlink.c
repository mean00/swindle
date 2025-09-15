/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#include "bmp_hosted.h"
#include "general.h"
#include "jep106.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_probe.h"
#include "wchlink.h"

void riscv_dtm_wchlink_init(riscv_dm_s *dtm);

static bool riscv_dtm_wchlink_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    (void)dmi;
    return wchlink_riscv_dmi_read(address, value);
}

static bool riscv_dtm_wchlink_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    (void)dmi;
    return wchlink_riscv_dmi_write(address, value);
}

uint8_t riscv_dtm_wchlink_handler()
{
    riscv_dmi_s *dmi_bus = calloc(1, sizeof(*dmi_bus));
    if (!dmi_bus)
    {
        DEBUG_WARN("calloc: failed in %s\n", __func__);
        return 0;
    }
    dmi_bus->read = riscv_dtm_wchlink_dmi_read;
    dmi_bus->write = riscv_dtm_wchlink_dmi_write;
    dmi_bus->version = RISCV_DEBUG_0_13;
    dmi_bus->designer_code = NOT_JEP106_MANUFACTURER_WCH; /* fixme: WCHLINK deosn't seem to have a jep106 code assigned
                                                             this is a dummy code */

    /* Call higher level code to discover the DMI bus */
    riscv_dmi_init(dmi_bus);
    uint8_t c = dmi_bus->ref_count;
    if (!dmi_bus->ref_count)
    {
        free(dmi_bus);
    }
    return c;
}
