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

#define RISCV_REG_A0 10
#define RISCV_REG_A1 11
#define RISCV_REG_A2 12
#define RISCV_REG_A3 13
#define RISCV_REG_PC 32
#define RISCV_REG_SP 2

/*
    Small helper function to translate target to hart and simplify parameters
    We assume it is 32 bits
*/
static bool riscv32_target_csr_write(target_s *target, const uint16_t reg, uint32_t val)
{
    riscv_hart_s *const hart = riscv_hart_struct(target);
    return riscv_csr_write(hart, reg, &val);
}

/*
    Small helper function to translate target to hart and simplify parameters
    We assume it is 32 bits
*/
static bool riscv32_target_csr_read(target_s *target, const uint16_t reg, uint32_t *val)
{
    riscv_hart_s *const hart = riscv_hart_struct(target);
    return riscv_csr_read(hart, reg, val);
}

/*
    Execute code on the target with the signature void function(a,b,c,d)
        - codexec is the address the code to tun is located at
        - param1/2/3/4 will end up as the 4 parameters of the stub function

    The flashstub must not use the stack at all.
    It returns true on success, false on error
    There is a built-in timeout of 10 seconds
*/
bool riscv32_run_stub(target_s *t, uint32_t codeexec, uint32_t param1, uint32_t param2, uint32_t param3,
                      uint32_t param4)
{
    bool ret = false;
    uint32_t pc, mie, zero = 0;
    // save PC & MIE
    t->reg_read(t, RISCV_REG_PC, &pc, 4);
    riscv32_target_csr_read(t, RV_CSR_MIE, &mie);
    riscv32_target_csr_write(t, RV_CSR_MIE, zero); // disable interrupt
    t->reg_write(t, RISCV_REG_A0, &param1, 4);
    t->reg_write(t, RISCV_REG_A1, &param2, 4);
    t->reg_write(t, RISCV_REG_A2, &param3, 4);
    t->reg_write(t, RISCV_REG_A3, &param4, 4);
    t->reg_write(t, RISCV_REG_PC, &codeexec, 4);

    target_halt_reason_e reason = TARGET_HALT_RUNNING;
    t->halt_resume(t, false); // go!
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 10000);
    while (reason == TARGET_HALT_RUNNING)
    {
        if (platform_timeout_is_expired(&timeout))
        {
            goto the_end;
        }
        reason = t->halt_poll(t, NULL);
    }

    if (reason == TARGET_HALT_ERROR)
    {
        goto the_end;
    }

    if (reason != TARGET_HALT_REQUEST)
    {
        goto the_end;
    }
    ret = true;
the_end:
    t->halt_request(t);
    if (ret)
    {
        uint32_t a0;
        t->reg_read(t, RISCV_REG_A0, &a0, 4);
        ret = (a0 == 0);
    }
    // restore PC & MIE
    riscv32_target_csr_write(t, RV_CSR_MIE, mie); // put back MIE
    t->reg_write(t, RISCV_REG_PC, &pc, 4);
    return ret;
}
//----
