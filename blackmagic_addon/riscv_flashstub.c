
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
/**
    \brief Execute code on the target to speed up flash operation
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
