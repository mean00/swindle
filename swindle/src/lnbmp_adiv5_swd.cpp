/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2020- 2021 Uwe Bonnes (bon@elektron.ikp.physik.tu-darmstadt.de)
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
 * This file implements the SWD specific functions of the
 * ARM Debug Interface v5 Architecture Specification, ARM doc IHI0031A.
 */
extern "C" {
#include "general.h"
#include "exception.h"
#include "adiv5.h"
#include "swd.h"
#include "target.h"
#include "target_internal.h"
}
extern uint32_t swd_delay_cnt ;
#include "lnGPIO.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"

extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK; // automatically add delay after toggle
extern SwdReset pReset;


extern void swdioSetAsOutput(bool output);

/**
 *
 *
 */
static uint8_t xmake_packet_request(uint8_t RnW, uint16_t addr)
{
	bool APnDP = addr & ADIV5_APnDP;

	addr &= 0xffU;

	uint8_t request = 0x81U; /* Park and Startbit */

	if (APnDP)
		request ^= 0x22U;
	if (RnW)
		request ^= 0x24U;

	addr &= 0xcU;
	request |= (addr << 1U) & 0x18U;
	if (addr == 4U || addr == 8U)
		request ^= 0x20U;

	return request;
}



/* Provide bare DP access functions without timeout and exception */
/**
 *
 *
 */
extern "C" void swd_line_reset_sequence(const bool idle_cycles)
{

/*
	 * A line reset is achieved by holding the SWDIOTMS HIGH for at least 50 SWCLKTCK cycles, followed by at least two idle cycles
	 * Note: in some non-conformant devices (STM32) at least 51 HIGH cycles and/or 3/4 idle cycles are required
	 *
	 * for robustness, we use 60 HIGH cycles and 4 idle cycles
	 */
    swdioSetAsOutput(true);
    pSWDIO.set(1);
    for (int i = 0; i < 32+28; i++)
    {
        pSWCLK.pulseOn();
    }
    if(idle_cycles)
    {
        pSWDIO.set(0);
        pSWCLK.pulseOn();
        pSWCLK.pulseOn();
        pSWCLK.pulseOn();
        pSWCLK.pulseOn();
    }
  //swd_proc.seq_out(0xffffffffU, 32U);                     /* 32 cycles HIGH */
	  //swd_proc.seq_out(0x0fffffffU, idle_cycles ? 32U : 28U); /* 28 cycles HIGH + 4 idle cycles if idle is requested */
}

/**
 *
 *
 *
 */
extern "C" bool xadiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
  swdioSetAsOutput(true);
	uint8_t request = xmake_packet_request(ADIV5_LOW_WRITE, addr);
  int parity = __builtin_popcount(data) & 1;
	//swd_proc.seq_out(request, 8U);
  for (int i = 0; i < 8; i++)
    {
        pSWDIO.set(request & 1);
        pSWCLK.pulseOn();
        request >>= 1;
    }

    swdioSetAsOutput(false);
    uint32_t res=0;
    uint32_t index=1;
    uint32_t bit;
    for(int i=0;i<3;i++)
    {
        bit = pSWDIO.read();
        if (bit)
            res |= index;
        pSWCLK.pulseOn();
        index <<= 1;
    }
    swdioSetAsOutput(true);
    uint32_t cpy=data;
    for(int i=0;i<32;i++)
    {
        pSWDIO.set(cpy & 1);
        pSWCLK.pulseOn();
        cpy >>= 1;
    }
    pSWDIO.set(parity);
    pSWCLK.pulseOn();
	//const uint8_t res = swd_proc.seq_in(3U);
	//swd_proc.seq_out_parity(data, 32U);
    pSWDIO.off();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
	//swd_proc.seq_out(0, 8U);
	return res != SWDP_ACK_OK;
}
/**
 *
 *
 *
 */
extern "C" uint32_t xadiv5_swd_read_no_check(const uint16_t addr)
{
  swdioSetAsOutput(true);
	uint8_t request = xmake_packet_request(ADIV5_LOW_WRITE, addr);
	//swd_proc.seq_out(request, 8U);
  for (int i = 0; i < 8; i++)
    {
        pSWDIO.set(request & 1);
        pSWCLK.pulseOn();
        request >>= 1;
    }

    swdioSetAsOutput(false);
    uint32_t res=0;
    uint32_t index=1;
    uint32_t bit;
    for(int i=0;i<3;i++)
    {
        bit = pSWDIO.read();
        if (bit)
            res |= index;
        pSWCLK.pulseOn();
        index <<= 1;
    }
    uint32_t cpy=0;
    index=1;
    for(int i=0;i<32;i++)
    {
        bit = pSWDIO.read();
        if (bit)
            cpy |= index;
        pSWCLK.pulseOn();
        index <<= 1;
    }
    int parity = __builtin_popcount(cpy) & 1;
    bit = pSWDIO.read();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
	//const uint8_t res = swd_proc.seq_in(3U);
	//swd_proc.seq_out_parity(data, 32U);
    swdioSetAsOutput(true);
    pSWDIO.off();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    pSWCLK.pulseOn();
    return cpy;
	//swd_proc.seq_out(0, 8U);
/*
	return res != SWDP_ACK_OK;
}	const uint8_t request = xmake_packet_request(ADIV5_LOW_READ, addr);
	swd_proc.seq_out(request, 8U);
	const uint8_t res = swd_proc.seq_in(3U);
	uint32_t data = 0;
	swd_proc.seq_in_parity(&data, 32U);
	swd_proc.seq_out(0, 8U);
	return res == SWDP_ACK_OK ? data : 0;
  */
}
/**
 *
 *
 *
 */
extern "C" uint32_t xadiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t RnW, const uint16_t addr, const uint32_t value)
{
	if ((addr & ADIV5_APnDP) && dp->fault)
		return 0;

	const uint8_t request = xmake_packet_request(RnW, addr);
	uint32_t response = 0;
	uint8_t ack = SWDP_ACK_WAIT;
	platform_timeout_s timeout;
	platform_timeout_set(&timeout, 250U);
	do {
		swd_proc.seq_out(request, 8U);
		ack = swd_proc.seq_in(3U);
		if (ack == SWDP_ACK_FAULT) {
			DEBUG_ERROR("SWD access resulted in fault, retrying\n");
			/* On fault, abort the request and repeat */
			/* Yes, this is self-recursive.. no, we can't think of a better option */
			adiv5_dp_write(dp, ADIV5_DP_ABORT,
				ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
					ADIV5_DP_ABORT_STKCMPCLR);
		}
	} while ((ack == SWDP_ACK_WAIT || ack == SWDP_ACK_FAULT) && !platform_timeout_is_expired(&timeout));

	if (ack == SWDP_ACK_WAIT) {
		DEBUG_ERROR("SWD access resulted in wait, aborting\n");
		dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
		dp->fault = ack;
		return 0;
	}

	if (ack == SWDP_ACK_FAULT) {
		DEBUG_ERROR("SWD access resulted in fault\n");
		dp->fault = ack;
		return 0;
	}

	if (ack == SWDP_ACK_NO_RESPONSE) {
		DEBUG_ERROR("SWD access resulted in no response\n");
		dp->fault = ack;
		return 0;
	}

	if (ack != SWDP_ACK_OK) {
		DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
		raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
	}

	if (RnW) {
		if (swd_proc.seq_in_parity(&response, 32U)) { /* Give up on parity error */
			dp->fault = 1U;
			DEBUG_ERROR("SWD access resulted in parity error\n");
			raise_exception(EXCEPTION_ERROR, "SWD parity error");
		}
	} else
		swd_proc.seq_out_parity(value, 32U);

	/* ARM Debug Interface Architecture Specification ADIv5.0 to ADIv5.2
	 * tells to clock the data through SW-DP to either :
	 * - immediate start a new transaction
	 * - continue to drive idle cycles
	 * - or clock at least 8 idle cycles
	 *
	 * Implement last option to favour correctness over
	 *   slight speed decrease
	 */
	swd_proc.seq_out(0, 8U);

	return response;
}

//--
