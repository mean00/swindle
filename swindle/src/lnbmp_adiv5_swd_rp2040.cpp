/*
 *  This is a streamlined version of adiv5_swd.c
 */
// clang-format off
extern "C"
{
#include "exception.h"
#include "general.h"
#include "adiv5.h"
#include "swd.h"
#include "target.h"
#include "target_internal.h"
}
#include "lnGPIO.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
#include "ln_rp_pio.h"
// clang-format on
extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK; // automatically add delay after toggle
#include "pio_swd.h"
extern rpPIO_SM *xsm;

#define LN_PARITY(x) __builtin_parity(x)
#if 1
    #define dLogger Logger
#else
    #define dLogger(....) {}
#endif
/**
 *
 */
static void zwrite(uint32_t size, uint32_t value)
{
    uint32_t zsize = ((size - 1) << 1) | 1;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->write(1, &value);
}
/**
 *
 */
static uint32_t zread(uint32_t size)
{
    uint32_t zsize = ((size - 1) << 1) | 0;
    uint32_t value = 0;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->waitRxReady();
    xsm->read(1, &value);
    return value>>(32-size);
}
/**
 *
 */
extern "C"
{
    void adiv5_init()
    {
    }
    /**
     * @brief
     *
     * @param RnW
     * @param addr
     * @return uint8_t
     */
    uint8_t xmake_packet_request(uint8_t RnW, uint16_t addr)
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

    /**
     * @brief
     *
     * @param idle_cycles
     */
    void swd_line_reset_sequence(const bool idle_cycles)
    {
        /*
         * A line reset is achieved by holding the SWDIOTMS HIGH for at least 50 SWCLKTCK cycles, followed by at least
         * two idle cycles Note: in some non-conformant devices (STM32) at least 51 HIGH cycles and/or 3/4 idle cycles
         * are required
         *
         * for robustness, we use 60 HIGH cycles and 4 idle cycles
         */
        zwrite(32, 0xFFFFFFFF);
        int pulse = 28;
        if (idle_cycles)
            pulse = 32;
        zwrite(pulse, 0x0FFF0000 + 0xFFFF);
    }
    /**
     * @brief
     *
     * @param access
     * @param addr
     * @return int
     */
    int preamble_w(uint8_t access, const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(access, addr);
        zwrite(8, request);
        uint32_t raw= zread(5);
        return raw >> 1; // one extra bit for turn (lsb)
    }
    int preamble_r(uint8_t access, const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(access, addr);
        zwrite(8, request);
        uint32_t raw=zread(4);
        dLogger("Raw reply =0x%x\n",raw);
        return raw >> 1;
    }

    /**
     * @brief
     *
     * @param addr
     * @param data
     * @return true
     * @return false
     */
    bool LN_FAST_CODE adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
    {
        int parity = __builtin_popcount(data) & 1;
        uint32_t res = preamble_w(ADIV5_LOW_WRITE, addr);
        if (res != SWDP_ACK_OK)
        {
            dLogger("SWD_WRITE: reply not ok 0x%x\n", res);
        }
        zwrite(32, data);
        // par
        zwrite(1, parity);
        //-- trailing bits
        zwrite(8, 0);
        return res != SWDP_ACK_OK;
    }

    /**
     * @brief
     *
     * @param addr
     * @return uint32_t
     */
    uint32_t LN_FAST_CODE adiv5_swd_read_no_check(const uint16_t addr)
    {
        uint32_t ret = preamble_r(ADIV5_LOW_READ, addr);
        if (ret != SWDP_ACK_OK)
        {
            dLogger("SWD_REAd: reply not ok 0x%x\n", ret);
        }
        //
        //-----
        ret = zread(32);
        int oparity = zread(1);
        int parity = __builtin_popcount(ret) & 1;
        if (parity != oparity)
        {
            dLogger("SWD:Wrong read parity\n");
        }
        zwrite(8, 0);
        return ret;
    }
    static int checkReply(adiv5_debug_port_s *dp, uint32_t ack)
    {
        ack&=7; // dafuq ?
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

    /**
     */
    void adiv_abort_current(adiv5_debug_port_s *dp)
    {

        // low_access(dp, ADIV5_LOW_WRITE, addr, value);
        //  no need for retry (?)
        adiv5_swd_write_no_check(ADIV5_DP_ABORT, ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR |
                                                     ADIV5_DP_ABORT_STKERRCLR | ADIV5_DP_ABORT_STKCMPCLR);
        // adiv5_swd_raw_access(dp, ADIV5_DP_ABORT,
        //                         ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
        //                         ADIV5_DP_ABORT_STKCMPCLR);
    }

    void LN_FAST_CODE adiv5_raw_write(const uint32_t ticks, const uint32_t value)
    {
        zwrite(ticks, value);
    }

    uint32_t LN_FAST_CODE adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                               const uint32_t value)
    {
        if ((addr & ADIV5_APnDP) && dp->fault)
            return 0;
        uint32_t ack = SWDP_ACK_WAIT;
        switch (rnw)
        {
        case ADIV5_LOW_READ: { // read or ...
                               //_______________________________________________________
            platform_timeout_s timeout;
            platform_timeout_set(&timeout, 250U);
            do
            {
                ack = preamble_r(ADIV5_LOW_READ, addr);
                dLogger("ReadAck = 0x%x\n",ack);
                if (ack == SWDP_ACK_FAULT)
                {
                    DEBUG_ERROR("SWD access resulted in fault, retrying\n");
                    /* On fault, abort the request and repeat */
                    /* Yes, this is self-recursive.. no, we can't think of a better option */
                    adiv_abort_current(dp);
                }
            } while ((ack == SWDP_ACK_WAIT || ack == SWDP_ACK_FAULT) && !platform_timeout_is_expired(&timeout));
            if(!checkReply(dp, ack)) //<=
            {
                dLogger("Bailing out\n");
                return 0;
            }
            uint32_t response = zread(32);
            int oparity = zread(1);
            int parity = __builtin_popcount(response) & 1;
            if (parity != oparity)
            {
                dLogger("SWD:Wrong read parity\n");
                dp->fault = 1;
                raise_exception(EXCEPTION_ERROR, "SWD read parity error");
            }
            zwrite(8, 0); // trailing bits
            return response;
        }
        break;
        case ADIV5_LOW_WRITE: { // write
                                //_______________________________________________________
            platform_timeout_s timeout;
            platform_timeout_set(&timeout, 250U);
            do
            {
                ack = preamble_w(ADIV5_LOW_READ, addr);
                if (ack == SWDP_ACK_FAULT)
                {
                    DEBUG_ERROR("SWD access resulted in fault, retrying\n");
                    /* On fault, abort the request and repeat */
                    /* Yes, this is self-recursive.. no, we can't think of a better option */
                    adiv_abort_current(dp);
                }
            } while ((ack == SWDP_ACK_WAIT || ack == SWDP_ACK_FAULT) && !platform_timeout_is_expired(&timeout));
            if (!checkReply(dp, ack))
            {
                return 0;
            }
            int parity = __builtin_popcount(value) & 1;
            zwrite(32, value);
            zwrite(1, parity);
            zwrite(8, 0); // trailing bits
            return 0;
        }
        break;
        default:
            //______________________________________________
            xAssert(0);
            break;
        }
    }

// uint32_t swd_delay_cnt = 1;
#if 0
/**
 * @brief
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
}
/**
 * @brief
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}
#endif
    extern void gmp_gpio_init_adc();
    /**

    */
    void bmp_gpio_init()
    {

        gmp_gpio_init_adc();
    }
    /**
     * @brief
     *
     */
    void bmp_io_begin_session()
    {
    }
    /**
     * @brief
     *
     */
    void bmp_io_end_session()
    {
    }
}
//--
