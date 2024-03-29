/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
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

#ifndef PLATFORMS_HOSTED_WCHLINK_H
#define PLATFORMS_HOSTED_WCHLINK_H

#include <stdbool.h>
#include "bmp_hosted.h"

#if HOSTED_BMP_ONLY == 1
bool wchlink_init()
{
	return false;
}

const char *wchlink_target_voltage()
{
	
	return "ERROR";
}

void wchlink_nrst_set_val(bool assert)
{
	(void)assert;
}

bool wchlink_nrst_get_val()
{
	return true;
}

uint32_t wchlink_rvswd_scan()
{	
	return 0;
}

bool wchlink_riscv_dmi_read(uint32_t address, uint32_t *value)
{
	(void)address;
	(void)value;
	return false;
}

bool wchlink_riscv_dmi_write( uint32_t address, uint32_t value)
{
	(void)address;
	(void)value;
	return false;
}

#else
bool wchlink_init();
const char *wchlink_target_voltage();
void wchlink_nrst_set_val(bool assert);
bool wchlink_nrst_get_val();
uint32_t wchlink_rvswd_scan();
bool wchlink_riscv_dmi_read( uint32_t address, uint32_t *value);
bool wchlink_riscv_dmi_write( uint32_t address, uint32_t value);
bool wchlink_riscv_dmi_read( uint32_t address, uint32_t *value);
bool wchlink_riscv_dmi_write( uint32_t address, uint32_t value);
#endif

#endif /* PLATFORMS_HOSTED_WCHLINK_H */
