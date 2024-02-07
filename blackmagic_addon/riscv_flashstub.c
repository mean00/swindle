/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2023 1BitSquared <info@1bitsquared.com>
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
#include "general.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_internal.h"

#if defined(PLATFORM_HAS_DEBUG)
#define debug DEBUG_WARN
#else
#define debug(...)                                                                                                     \
    {                                                                                                                  \
    }
#endif
/*
    Execute code on the target to speed up flash operation
    at the end of the execution, the code jumps back to the beginning of the RAM where
    we put a breakpoint in _prepare.
*/
bool riscv32_run_stub(target_s *t, uint32_t codeexec, uint32_t param1, uint32_t param2, uint32_t param3,
                      uint32_t temporary_stack)
{
    bool ret = false;
    uint32_t pc, sp, mie, zero = 0;
    // save PC & SP
    t->reg_read(t, RISCV_REG_PC, &pc, 4);
    t->reg_read(t, RISCV_REG_SP, &sp, 4);
    t->reg_read(t, RISCV_REG_MIE, &mie, 4);

    t->reg_write(t, RISCV_REG_MIE, &zero, 4); // disable interrupt
    t->reg_write(t, RISCV_REG_A0, &param1, 4);
    t->reg_write(t, RISCV_REG_A1, &param2, 4);
    t->reg_write(t, RISCV_REG_A2, &param3, 4);
    t->reg_write(t, RISCV_REG_SP, &temporary_stack, 4);
    t->reg_write(t, RISCV_REG_PC, &codeexec, 4);

    target_halt_reason_e reason = TARGET_HALT_RUNNING;
    t->halt_resume(t, false); // go!
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 10000);
    while (reason == TARGET_HALT_RUNNING)
    {
        if (platform_timeout_is_expired(&timeout))
        {
            debug("Timeout executing code !\n");
            goto the_end;
        }
        reason = t->halt_poll(t, NULL);
    }

    if (reason == TARGET_HALT_ERROR)
    {
        debug("Error executing code !\n");
        goto the_end;
    }

    if (reason != TARGET_HALT_REQUEST)
    {
        debug("OOPS executing code !\n");
        goto the_end;
    }
    ret = true;
the_end:
    t->halt_request(t);
    if (!ret)
        debug("Exec error\n");
    else
    {
        uint32_t a0;
        t->reg_read(t, RISCV_REG_A0, &a0, 4);
        ret = (a0 == 0);
    }
    // restore PC & SP & MIE
    t->reg_write(t, RISCV_REG_MIE, &mie, 4);
    t->reg_write(t, RISCV_REG_PC, &pc, 4);
    t->reg_write(t, RISCV_REG_SP, &sp, 4);

    return ret;
}
