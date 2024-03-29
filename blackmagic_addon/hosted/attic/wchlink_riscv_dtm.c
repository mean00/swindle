/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
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

#include "general.h"
#include "jep106.h"
#include "riscv_debug.h"
#include "wchlink_protocol.h"

static void wchlink_riscv_dtm_init(riscv_dmi_s *dmi);
static bool wchlink_riscv_dmi_read(riscv_dmi_s *dmi, uint32_t address, uint32_t *value);
static bool wchlink_riscv_dmi_write(riscv_dmi_s *dmi, uint32_t address, uint32_t value);

uint8_t  wchlink_riscv_dtm_handler(void)
{
	riscv_dmi_s *dmi = calloc(1, sizeof(*dmi));
	if (!dmi) { /* calloc failed: heap exhaustion */
		DEBUG_WARN("calloc: failed in %s\n", __func__);
		return -1;
	}

	wchlink_riscv_dtm_init(dmi);
	/* If we failed to find any DMs or Harts, free the structure */
	if (!dmi->ref_count)
		free(dmi);
	return 0;
}

static void wchlink_riscv_dtm_init(riscv_dmi_s *const dmi)
{
	/* WCH-Link doesn't have any mechanism to identify the DTM manufacturer, so we'll just assume it's WCH */
	dmi->designer_code = NOT_JEP106_MANUFACTURER_WCH;

	dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */

	/* WCH-Link has a fixed address width of 8 bits, limited by the USB protocol (is RVSWD also fixed?) */
	dmi->address_width = 8U;

	dmi->read = wchlink_riscv_dmi_read;
	dmi->write = wchlink_riscv_dmi_write;

	riscv_dmi_init(dmi);
}

static bool wchlink_riscv_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
	uint8_t status = 0;
	const bool result = wchlink_transfer_dmi(RV_DMI_READ, address, 0, value, &status);

	/* Translate error 1 into RV_DMI_FAILURE per the spec, also write RV_DMI_FAILURE if the transfer failed */
	dmi->fault = !result || status == 1U ? RV_DMI_FAILURE : status;
	return dmi->fault == RV_DMI_SUCCESS;
}

static bool wchlink_riscv_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
	uint8_t status = 0;
	const bool result = wchlink_transfer_dmi(RV_DMI_WRITE, address, value, NULL, &status);

	/* Translate error 1 into RV_DMI_FAILURE per the spec, also write RV_DMI_FAILURE if the transfer failed */
	dmi->fault = !result || status == 1U ? RV_DMI_FAILURE : status;
	return dmi->fault == RV_DMI_SUCCESS;
}
