/*

 */
#include "esprit.h"
#include "bmp_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_swdio_esp.h"
extern void gmp_gpio_init_adc();

#include "bmp_tap_esp.h"
#if PC_HOSTED == 0
#include "bmp_pinout.h"
#endif
#include "lnbmp_parity.h"
extern "C" void swdptap_init_stubs();
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
}
#define LN_READ_BIT(value)                                                                                             \
    rSWCLK->pulseClock();                                                                                              \
    uint32_t bit = rSWDIO->read();                                                                                     \
    if (bit)                                                                                                           \
        value |= index;                                                                                                \
    index <<= 1;

#define LN_WRITE_BIT(value)                                                                                            \
    rSWDIO->set(value & 1);                                                                                            \
    rSWCLK->pulseClock();

/**
 */
static void zwrite(uint32_t nbTicks, uint32_t val)
{
    xAssert(rSWDIO->dir());
    for (int i = 0; i < nbTicks; i++)
    {
        rSWDIO->set(val & 1);
        rSWCLK->pulseClock(); // off -> on ->> off
        val >>= 1;
    }
}
/**
 */
static uint32_t zread(uint32_t nbTicks)
{
    xAssert(!rSWDIO->dir());
    uint32_t index = 1;
    uint32_t val = 0;
    for (int i = 0; i < nbTicks; i++)
    {
        rSWCLK->clockOn(); // off-> on ->> off
        rSWCLK->wait();    // off-> on ->> off
        uint32_t bit = rSWDIO->read();
        rSWCLK->clockOff(); // off-> on ->> off

        if (bit)
            val |= index;
        index <<= 1;
    }
    return val;
}
/**
    \fn ln_adiv5_swd_write_no_check
 */
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    uint8_t request = make_packet_request(ADIV5_LOW_WRITE, addr);
    xAssert(rSWDIO->dir());
    zwrite(8, request);
    rSWDIO->dir_input();
    rSWCLK->pulseClock(); // turnaround
    uint32_t ack = zread(3);
    zread(1);
    bool parity = lnOddParity(data);
    rSWDIO->dir_output();
    zwrite(32, data);
    zwrite(1, parity);
    zwrite(8, 0); // idle
    return ack != SWD_ACK_OK;
}

/**
        \fn ln_adiv5_swd_read_no_check

 */
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr)
{
    uint8_t request = make_packet_request(ADIV5_LOW_READ, addr);
    uint32_t index;

    xAssert(rSWDIO->dir());
    zwrite(8, request);
    rSWDIO->dir_input();
    rSWCLK->pulseClock(); // turnaround
    uint32_t ack = zread(3);
    uint32_t data = zread(32);
    bool parity = zread(1);
    zread(1); // 2nd turnaround
    rSWDIO->dir_output();
    zwrite(8, 0); // back to idle
    return ack == SWD_ACK_OK ? data : 0;
}
/**
        \fn ln_adiv5_swd_raw_access

 */
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value)
{
    //
    if ((addr & ADIV5_APnDP) && dp->fault)
        return 0;

    const uint8_t request = make_packet_request(rnw, addr);
    uint8_t ack = SWD_ACK_WAIT;
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 250U);
    while (1)
    {
        xAssert(rSWDIO->dir());
        zwrite(8, request);
        rSWDIO->dir_input();
        uint32_t ack = zread(3);
        // HACKME
        bool expired = platform_timeout_is_expired(&timeout);
        if (ack == SWD_ACK_OK)
        {
            goto done;
        }
        // if we get here, we are retrying
        // switch back to output
        rSWDIO->dir_output();
        zwrite(2, 0x03);

        switch (ack)
        {
        case SWD_ACK_OK:
            xAssert(0);
            break;
        case SWD_ACK_NO_RESPONSE:
            DEBUG_ERROR("SWD access resulted in no response\n");
            dp->fault = ack;
            return 0;

        case SWD_ACK_WAIT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in wait, aborting\n");
                if (dp->fault == SWD_ACK_WAIT)
                    return 0;
                dp->fault = ack; // prevent recursion
                dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
                dp->fault = ack;
                return 0;
            }
            break;
        case SWD_ACK_FAULT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in fault\n");
                dp->fault = ack;
                return 0;
            }
            DEBUG_ERROR("SWD access resulted in fault, retrying\n");
            /* On fault, abort the request and repeat */
            /* Yes, this is self-recursive.. no, we can't think of a better option */
            adiv5_dp_write(dp, ADIV5_DP_ABORT,
                           ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
                               ADIV5_DP_ABORT_STKCMPCLR);
            break;
        default:
            DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
            raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
            break;
        }
    }
done:
    // We are out of read sequence, CLK is low
    if (rnw) // read ****************************************** HERE *************************
    {
        uint32_t index = 1;
        uint32_t response = zread(32);
        bool parityBit = zread(3) & 1;
        bool currentParity = lnOddParity(response);
        rSWDIO->dir_output();
        zwrite(8, 0);
        if (currentParity != parityBit)
        { /* Give up on parity error */
            dp->fault = 1U;
            DEBUG_ERROR("SWD access resulted in parity error\n");
            raise_exception(EXCEPTION_ERROR, "SWD parity error");
        }
        return response;
    }
    // write
    xAssert(!rSWDIO->dir());
    rSWDIO->dir_output();
    bool parity = lnOddParity(value);
    zwrite(2, 0x03);
    zwrite(32, value);
    zwrite(1, parity);
    zwrite(8, 0);
    return 0;
    //--
}
extern "C" void ln_raw_swd_reset(uint32_t pulses, uint32_t idle_cycles)
{
    rSWDIO->set(1);
    for (int i = 0; i < pulses; i++)
        rSWCLK->pulseClock();
    if (idle_cycles)
    {
        rSWDIO->set(0);
        for (int i = 0; i < 8; i++)
            rSWCLK->pulseClock();
        rSWDIO->set(1);
    }
}
/**
 */
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    rSWDIO->dir_output();
    zwrite(tick, value);
}
/**
 *
 */
void bmp_extraSetWaitState()
{
}
/*
 *
 *
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}

// EOF
