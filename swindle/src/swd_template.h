//____________________________________________
//____________________________________________
//____________________________________________
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    const uint8_t request = make_packet_request(ADIV5_LOW_WRITE, addr);
    zwrite(8, request);
    DIR_INPUT();
    const uint8_t res = (zread(4) >> 1) & 7; // turn +  reply
    SWD_WAIT_PERIOD();
    // zread(1);                          // turn
    DIR_OUTPUT();
    const bool parity = lnOddParity(data);
    zwrite(32, data);
    zwrite(9, parity);
    return res != SWD_ACK_OK;
}
//
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr)
{
    const uint8_t request = make_packet_request(ADIV5_LOW_READ, addr);
    zwrite(8, request);
    DIR_INPUT();
    const uint8_t res = zread(4) >> 1; // turn + reply
    uint32_t data = zread(32);
    bool parity = lnOddParity(data);
    const bool rd = zread(2) & 1; // parity + read
    if (rd != parity)
    {
        Logger("swd_read: wrong parity\n");
    }
    DIR_OUTPUT();
    zwrite(8, 0); // idle
    return res == SWD_ACK_OK ? data : 0;
}
//
static bool sendHeader(const uint8_t request, adiv5_debug_port_s *dp)
{
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 250U);
    uint8_t ack;
    do
    {
        zwrite(8, request);
        DIR_INPUT();
        ack = (zread(4) >> 1) & 7; // turn +  reply
        if (ack == SWD_ACK_OK)
            return true;
        if (ack == SWD_ACK_FAULT)
        {
            DEBUG_ERROR("SWD access resulted in fault, retrying\n");
            /* On fault, abort the request and repeat */
            /* Yes, this is self-recursive.. no, we can't think of a better option */
            //        swdptap_turnaround(SWDIO_STATUS_DRIVE);
            zread(1); // turn
            DIR_OUTPUT();
            //        switch_to_output();
            adiv5_dp_write(dp, ADIV5_DP_ABORT,
                           ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
                               ADIV5_DP_ABORT_STKCMPCLR);
        }
        // something is wrong, retry
        zread(1); // turn
        DIR_OUTPUT();
        zwrite(8, 0);
    } while ((ack == SWD_ACK_WAIT || ack == SWD_ACK_FAULT) && !platform_timeout_is_expired(&timeout));

    if (ack == SWD_ACK_WAIT)
    {
        DEBUG_ERROR("SWD access resulted in wait, aborting\n");
        dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
        dp->fault = ack;
        return false;
    }

    if (ack == SWD_ACK_FAULT)
    {
        DEBUG_ERROR("SWD access resulted in fault\n");
        dp->fault = ack;
        return false;
    }

    if (ack == SWD_ACK_NO_RESPONSE)
    {
        DEBUG_ERROR("SWD access resulted in no response\n");
        dp->fault = ack;
        return false;
    }

    if (ack != SWD_ACK_OK)
    {
        DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
        raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
    }
    return true;
}
//
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value)
{
    if ((addr & ADIV5_APnDP) && dp->fault)
        return 0;

    const uint8_t request = make_packet_request(rnw, addr);
    // read
    if (rnw)
    {
        if (!sendHeader(request, dp))
        {
            return 0;
        }
        uint32_t response = zread(32);
        bool read_parity = zread(2) & 1; // parity + turn
        DIR_OUTPUT();
        zwrite(8, 0);
        bool parity = lnOddParity(response);
        if (read_parity != parity)
        {
            /* Give up on parity error */
            dp->fault = 1U;
            DEBUG_ERROR("SWD access resulted in parity error\n");
            raise_exception(EXCEPTION_ERROR, "SWD parity error");
        }
        return response;
    }
    // write
    if (!sendHeader(request, dp))
    {
        return 0;
    }
    zread(1);          // turn
    SWD_WAIT_PERIOD(); // extra
    DIR_OUTPUT();
    const bool parity = lnOddParity(value);
    zwrite(32, value);
    zwrite(9, parity);
    return 0;
}
//
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    DIR_OUTPUT();
    zwrite(tick, value);
}
// EOF
