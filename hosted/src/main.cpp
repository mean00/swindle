/*
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

/* Provides main entry point.  Initialise subsystems and enter GDB
 * protocol loop.
 */
extern "C"
{
#include "general.h"
#include "gdb_if.h"
#include "gdb_main.h"
#include "target.h"
#include "exception.h"
#include "gdb_packet.h"
#include "morse.h"
}

#define BUF_SIZE 1024U
static char pbuf[BUF_SIZE + 1U];

static void bmp_poll_loop(void)
{
	SET_IDLE_STATE(false);
	while (gdb_target_running && cur_target) {
		gdb_poll_target();

		// Check again, as `gdb_poll_target()` may
		// alter these variables.
		if (!gdb_target_running || !cur_target)
			break;
		char c = gdb_if_getchar_to(0);
		if (c == '\x03' || c == '\x04')
			target_halt_request(cur_target);
		platform_pace_poll();
#ifdef ENABLE_RTT
		if (rtt_enabled)
			poll_rtt(cur_target);
#endif
	}

	SET_IDLE_STATE(true);
	size_t size = gdb_getpacket(pbuf, BUF_SIZE);
	// If port closed and target detached, stay idle
	if (pbuf[0] != '\x04' || cur_target)
		SET_IDLE_STATE(false);
	gdb_main(pbuf, sizeof(pbuf), size);
}

int main(int argc, char **argv)
{
	platform_init(argc, argv);

	while (true) {
		volatile struct exception e;
		TRY_CATCH(e, EXCEPTION_ALL) {
			bmp_poll_loop();
		}
		if (e.type) {
			gdb_putpacketz("EFF");
			target_list_free();
			morse("TARGET LOST.", 1);
		}
	}

	/* Should never get here */
	return 0;
}

