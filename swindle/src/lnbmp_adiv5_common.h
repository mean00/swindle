
#pragma once

/**
 */
static inline uint32_t lnOddParity(uint32_t value)
{
    value ^= value >> 16;
    value ^= value >> 8;
    value ^= value >> 4;
    value &= 0xf;
    return (0x6996 >> value) & 1;
}
/**
 */
void adiv_abort_current(adiv5_debug_port_s *dp)
{
    // We change a bit the sequence here to avoid recursion
    // not completely sure it is ok
    // low_access(dp, ADIV5_LOW_WRITE, addr, value);

    adiv5_swd_write_no_check(ADIV5_DP_ABORT, ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR |
                                                 ADIV5_DP_ABORT_STKERRCLR | ADIV5_DP_ABORT_STKCMPCLR);
    // adiv5_swd_raw_access(dp, ADIV5_DP_ABORT,
    //                         ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
    //                         ADIV5_DP_ABORT_STKCMPCLR);
}
static int checkReply(adiv5_debug_port_s *dp, uint32_t ack)
{
    ack &= 7; // dafuq ?
    switch (ack)
    {
    case SWDP_ACK_WAIT: {
        dLogger("SWD access resulted in wait, aborting\n");
        dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
        dp->fault = ack;
        return 0;
        break;
    }

    case SWDP_ACK_FAULT: {
        dLogger("SWD access resulted in fault\n");
        dp->fault = ack;
        return 0;
        break;
    }

    case SWDP_ACK_NO_RESPONSE: {
        dLogger("SWD access resulted in no response\n");
        dp->fault = ack;
        return 0;
        break;
    }
    case SWDP_ACK_OK:
        return 1;
        break;
    default: {
        dLogger("SWD access has invalid ack %x\n", ack);
        raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
        return 0;
        break;
    }
    }
}