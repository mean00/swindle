/**
 * @file remote_rv_protocol.c
 * @author your name (you@domain.com)
 * @brief
 * @version 0.1
 * @date 2024-01-28
 *
 * @copyright Copyright (c) 2024
 *
 */
#include "general.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_internal.h"

#include "bmp_remote.h"
#include "gdb_packet.h"
#include "remote/protocol_v2_defs.h"

extern bool remote_adiv5_swd_write_no_check_rs(const uint16_t addr, const uint32_t data);
extern void remote_raw_swd_write_rs(uint32_t tick, uint32_t value);
extern uint32_t remote_adiv5_swd_read_no_check_rs(const uint16_t addr);
extern uint32_t remote_adiv5_swd_raw_access_rs(const uint8_t rnw, const uint16_t addr, const uint32_t value,
                                               uint32_t *fault);

#if 1
#define LOGG(...)                                                                                                      \
    {                                                                                                                  \
    }
#else
#define LOGG printf
#endif

void __attribute__((noreturn)) do_assert(const char *a);
#define xAssert(x) do_assert("")

bool remote_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    return remote_adiv5_swd_write_no_check_rs(addr, data);
}
uint32_t remote_adiv5_swd_read_no_check(const uint16_t addr)
{
    return remote_adiv5_swd_read_no_check_rs(addr);
}
uint32_t remote_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                     const uint32_t value)
{
    uint32_t fault = 0;
    uint32_t reply = remote_adiv5_swd_raw_access_rs(rnw, addr, value, &fault);
    LOGG("** FAULT : %d\n", fault);
    switch (fault)
    {
    case 0: // it means ok, else we have either reply to ack or 0xff
        LOGG("  *** VALUE=0x%x\n", reply);
        return reply;
        break;
    case 0xff: // raise
        raise_exception(EXCEPTION_ERROR, "Low level remote exception");
        break;
    case SWD_ACK_OK:
        xAssert(0); // should not happen, ok is zero!
        break;
    case SWD_ACK_WAIT:
        dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
        dp->fault = fault;
        break;
    case SWD_ACK_NO_RESPONSE: // 0x07U
    default:
        dp->fault = fault;
        break;
    }
    return 0;
}
void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    return remote_raw_swd_write_rs(tick, value);
}
