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
// clang-format on
extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK; // automatically add delay after toggle
extern SwdReset pReset;

extern "C" uint32_t old_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" bool old_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern void swdioSetAsOutput(bool output);
extern "C"
{
    /**
     * @brief
     *
     * @param RnW
     * @param addr
     * @return uint8_t
     */
    static uint8_t LN_FAST_CODE xmake_packet_request(uint8_t RnW, uint16_t addr)
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
        swdioSetAsOutput(true);
        pSWDIO.set(1);
        for (int i = 0; i < 32 + 28; i++)
        {
            pSWCLK.pulseClock();
        }
        if (idle_cycles)
        {
            pSWDIO.set(0);
            for (int i = 0; i < 4; i++)
            {
                pSWCLK.pulseClock();
            }
        }
    }
    /**
     * @brief
     *
     * @param access
     * @param addr
     * @return int
     */
    static int LN_FAST_CODE preamble(uint8_t access, const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(access, addr);
        swdioSetAsOutput(true);
        pSWCLK.clockOff();
        for (int i = 0; i < 8; i++)
        {
            pSWDIO.set(request & 1);
            pSWCLK.pulseClock();
            request >>= 1;
        }
        //--
        uint32_t index = 1;
        uint32_t ret = 0;
        int bit;
        swdioSetAsOutput(false);
        pSWCLK.clockOff();
        for (int i = 0; i < 3; i++)
        {
            bit = pSWDIO.read();
            if (bit)
                ret |= index;
            pSWCLK.pulseClock();
            index <<= 1;
        }
        return ret;
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
        // return old_adiv5_swd_write_no_check(addr,data);
        int parity = __builtin_popcount(data) & 1;
        uint32_t res = preamble(ADIV5_LOW_WRITE, addr);
        if (res != SWDP_ACK_OK)
        {
            Logger("SWD_WRITE: reply not ok 0x%x\n", res);
        }

        uint32_t cpy = data;
        swdioSetAsOutput(true);
        pSWCLK.clockOff();
        for (int i = 0; i < 32; i++)
        {
            pSWDIO.set(cpy & 1);
            pSWCLK.pulseClock();
            cpy >>= 1;
        }
        // par
        pSWDIO.set(parity);
        pSWCLK.pulseClock();
        //--
        pSWCLK.clockOff();
        pSWDIO.off();

        for (int i = 0; i < 8; i++)
        {
            pSWCLK.pulseClock();
        }
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
        uint32_t ret = preamble(ADIV5_LOW_READ, addr);
        if (ret != SWDP_ACK_OK)
        {
            Logger("SWD_REAd: reply not ok 0x%x\n", ret);
        }
        //
        //-----
        int index = 1;
        int bit;
        ret = 0;
        pSWCLK.clockOff();
        for (int i = 0; i < 32; i++)
        {
            bit = pSWDIO.read();
            if (bit)
                ret |= index;
            pSWCLK.pulseClock();
            index <<= 1;
        }

        //
        swdioSetAsOutput(true);
        pSWCLK.clockOff();
        pSWDIO.off();
        for (int i = 0; i < 8; i++)
        {
            pSWCLK.pulseClock();
        }
        return ret;
    }
}
//--
