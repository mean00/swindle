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

#include "lnArduino.h"
#include "lnBMP_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 4;

#include "lnBMP_swdio.h"
#if PC_HOSTED == 0
#include "lnBMP_pinout.h"
#endif

#if 1
#define dLogger Logger
#else
#define dLogger(...)                                                                                                   \
    {                                                                                                                  \
    }
#endif
#include "lnbmp_adiv5_common.h"

static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);

static void swdioSetAsOutput(bool output);

SwdPin pSWDIO(TSWDIO_PIN);
SwdWaitPin pSWCLK(TSWDCK_PIN); // automatically add delay after toggle
SwdReset pReset(TRESET_PIN);

extern "C" uint32_t old_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" bool old_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern void swdioSetAsOutput(bool output);
extern swd_proc_s swd_proc;
#define LN_FAST_CODE

// default state is clockOn()
#define XIO_SET()                                                                                                      \
    {                                                                                                                  \
        pSWDIO.on();                                                                                                   \
    }
#define XIO_CLEAR()                                                                                                    \
    {                                                                                                                  \
        pSWDIO.off();                                                                                                  \
    }
#define XWRITE(x)                                                                                                      \
    {                                                                                                                  \
        pSWCLK.clockOff();                                                                                             \
        pSWDIO.set(x);                                                                                                 \
        pSWCLK.clockOn();                                                                                              \
    } // write is active on rising edge
#define XREAD(x)                                                                                                       \
    {                                                                                                                  \
        pSWCLK.clockOff();                                                                                             \
        x = pSWDIO.read();                                                                                             \
        pSWCLK.clockOn();                                                                                              \
    } // read is on falling edge
#define XINPUT()                                                                                                       \
    {                                                                                                                  \
        pSWDIO.input();                                                                                                \
    }
#define XOUTPUT()                                                                                                      \
    {                                                                                                                  \
        pSWDIO.output();                                                                                               \
    } // default is output
#define XSWD_REVPULSE()                                                                                                \
    {                                                                                                                  \
        pSWCLK.clockOff();                                                                                             \
        pSWCLK.clockOn();                                                                                              \
    }

extern "C"
{

    void LN_FAST_CODE adiv5_raw_write(const uint32_t ticks, const uint32_t value)
    {
        uint32_t val = value;
        XOUTPUT();
        for (int i = 0; i < ticks; i++)
        {
            XWRITE(val & 1);
            val >>= 1;
        }
    }
    uint32_t LN_FAST_CODE adiv5_raw_read_parity(const uint32_t ticks)
    {
        uint32_t bit;
        uint32_t val = 0;
        XINPUT();

        uint32_t result = 0;
        int dex = 0;

        for (int i = 0; i < ticks; i++)
        {
            XREAD(bit);
            result += bit << dex;
            dex++;
        }
        uint32_t parity = 0;
        XREAD(parity); // parity
        // TODO: Check parity ?
        return result;
    }

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
        XOUTPUT();
        XIO_SET();
        for (int i = 0; i < 32 + 28; i++)
        {
            XSWD_REVPULSE(); // send 60 clocks with IO high
        }
        if (idle_cycles)
        {
            XIO_CLEAR();
            for (int i = 0; i < 4; i++)
            {
                XSWD_REVPULSE(); // 4 more cycle with IO LOW
            }
        }
        XIO_SET(); // and back to one
    }
    /**
     * @brief
     *
     * @param access
     * @param addr
     * @return int
     */
    static int LN_FAST_CODE preamble_r(const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(ADIV5_LOW_READ, addr);

        for (int i = 0; i < 8; i++)
        {
            XWRITE(request & 1);
            request >>= 1;
        }
        // turnaround
        XWRITE(1);
        XINPUT();

        //--
        uint32_t rd_bit = 0;
        int result;
        XREAD(rd_bit);
        result = rd_bit;
        XREAD(rd_bit);
        result = result + (rd_bit << 1);
        XREAD(rd_bit);
        result = result + (rd_bit << 2);
        return result;
    }
    static int LN_FAST_CODE preamble_w(const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(ADIV5_LOW_WRITE, addr);

        for (int i = 0; i < 8; i++)
        {
            XWRITE(request & 1);
            request >>= 1;
        }
        // turnaround
        XWRITE(1);
        XINPUT();
        //--
        uint32_t rd_bit = 0;
        int result;
        XREAD(rd_bit);
        result = rd_bit;
        XREAD(rd_bit);
        result = result + (rd_bit << 1);
        XREAD(rd_bit);
        result = result + (rd_bit << 2);
        XOUTPUT();
        XWRITE(1); // 2nd turnaround
        return result;
    }

    /**
     * @brief  ** WRITE ** WRITE ** WRITE **
     *
     * @param addr
     * @param data
     * @return true
     * @return false
     */
    bool LN_FAST_CODE adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
    {
        uint32_t cpy = data;
        int parity = lnOddParity(data);
        uint32_t res = preamble_w(addr);
        if (res != SWDP_ACK_OK)
        {
            Logger("SWD_WRITE: reply not ok 0x%x\n", res);
            return false;
        }
        // body
        for (int i = 0; i < 32; i++)
        {
            XWRITE(cpy & 1);
            cpy >>= 1;
        }
        // parity
        XWRITE(parity);

        // trailing 8 0
        XWRITE(0); // all at 1..
        for (int i = 0; i < 7; i++)
        {
            XSWD_REVPULSE();
        }
        return true;
    }

    /**
     * @brief  ** READ ** READ ** READ **
     *
     * @param addr
     * @return uint32_t
     */
    uint32_t LN_FAST_CODE adiv5_swd_read_no_check(const uint16_t addr)
    {
        uint32_t ret = preamble_r(addr);
        if (ret != SWDP_ACK_OK)
        {
            Logger("SWD_REAd: reply not ok 0x%x\n", ret);
            return 0;
        }
        uint32_t result = 0;
        int bit = 0;
        int dex = 0;
        for (int i = 0; i < 32; i++)
        {
            XREAD(bit);
            result += bit << dex;
            dex++;
        }
        uint32_t oparity = 0;
        XREAD(oparity); // parity
        int parity = lnOddParity(result);

        if (parity != oparity)
        {
            Logger("SWD_READ: bad parity !\n");
            return 0;
        }
        XOUTPUT();
        // trailing 8 bit
        XWRITE(0); // all at 1..
        for (int i = 0; i < 7; i++)
        {
            XSWD_REVPULSE();
        }
        return result;
    }
    /**
                    RAW ACCESS
     */
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
                ack = preamble_r(addr);
                // dLogger("ReadAck = 0x%x\n", ack);
                if (ack == SWDP_ACK_FAULT)
                {
                    DEBUG_ERROR("SWD access resulted in fault, retrying\n");
                    /* On fault, abort the request and repeat */
                    /* Yes, this is self-recursive.. no, we can't think of a better option */
                    adiv_abort_current(dp);
                }
            } while ((ack == SWDP_ACK_WAIT || ack == SWDP_ACK_FAULT) && !platform_timeout_is_expired(&timeout));
            if (!checkReply(dp, ack)) //<=
            {
                dLogger("Bailing out\n");
                return 0;
            }
            //
            uint32_t response = 0;
            int dex = 0;
            int bit;
            for (int i = 0; i < 32; i++)
            {
                XREAD(bit);
                response += bit << dex;
                dex++;
            }
            uint32_t oparity = 0;

            XREAD(oparity); // parity
            XOUTPUT();
            // trailing 8 bit
            XWRITE(0); // all at 1..
            for (int i = 0; i < 7; i++)
            {
                XSWD_REVPULSE();
            }
            return response;
        }
        break;
        case ADIV5_LOW_WRITE: { // write
                                //_______________________________________________________
            platform_timeout_s timeout;
            platform_timeout_set(&timeout, 250U);
            do
            {
                ack = preamble_w(addr);
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
            // turnaround missing here!
            int parity = lnOddParity(value);
            uint32_t cpy = value;
            for (int i = 0; i < 32; i++)
            {
                XWRITE(cpy & 1);
                cpy >>= 1;
            }
            XWRITE(parity);
            // trailing 8 bit
            XWRITE(0); // all at 1..
            for (int i = 0; i < 7; i++)
            {
                XSWD_REVPULSE();
            }
            return 0;
        }
        break;
        default:
            //______________________________________________
            xAssert(0);
            break;
        }
    }
}

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

/**

*/
void bmp_gpio_init()
{
    pSWDIO.hiZ();
    pSWDIO.hiZ();
    pSWCLK.hiZ();
    pSWCLK.hiZ();
    pReset.hiZ(); // hi-z by default
    pReset.off(); // hi-z by default

    gmp_gpio_init_adc();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
    pSWDIO.on();
    pSWDIO.output();
    pSWCLK.clockOn();
    pSWCLK.output();
    pReset.off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    pSWDIO.hiZ();
    pSWDIO.hiZ();
    pSWCLK.hiZ();
    pSWCLK.hiZ();
    pReset.off(); // hi-z by default
}

/**
 */
static uint32_t SwdRead(size_t len)
{
    uint32_t index = 1;
    uint32_t ret = 0;
    int bit;

    swdioSetAsOutput(false);
    for (int i = 0; i < len; i++)
    {
        pSWCLK.clockOff();
        bit = pSWDIO.read();
        if (bit)
            ret |= index;
        pSWCLK.clockOn();
        index <<= 1;
    }
    pSWCLK.clockOff();
    return ret;
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    uint32_t res = 0;
    res = SwdRead(len);
    bool currentParity = __builtin_parity(res);
    bool parityBit = (pSWDIO.read() == 1);
    pSWCLK.clockOn();
    *ret = res;
    swdioSetAsOutput(true);
    return currentParity == parityBit; // should be equal
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    // int cnt;
    swdioSetAsOutput(true);
    for (int i = 0; i < ticks; i++)
    {
        pSWCLK.clockOff();
        pSWDIO.set(MS & 1);
        pSWCLK.clockOn();
        MS >>= 1;
    }
    pSWCLK.clockOff();
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    bool parity = __builtin_parity(MS);
    SwdWrite(MS, ticks);
    pSWDIO.set(parity);
    pSWCLK.clockOn();
    pSWCLK.clockOff();
}

/**
    properly invert SWDIO direction if needed
*/
static bool oldDrive = false;
void LN_FAST_CODE swdioSetAsOutput(bool output)
{
    if (output == oldDrive)
        return;
    oldDrive = output;

    switch ((int)output)
    {
    case false: // in
    {
        pSWDIO.input();
        pSWCLK.wait();
        pSWCLK.clockOn();
        break;
    }
    break;
    case true: // out
    {
        pSWCLK.clockOff();
        pSWCLK.clockOn();
        pSWCLK.clockOff();
        pSWDIO.output();
        break;
    }
    default:
        break;
    }
}

/**
 */
extern "C" void platform_nrst_set_val(bool assert)
{
    if (assert) // force reset to low
    {
        pReset.on();
    }
    else // release reset
    {
        pReset.off();
    }
}
/**
 */
extern "C" bool platform_nrst_get_val(void)
{
    return pReset.state();
}

swd_proc_s swd_proc;
/**

*/
extern "C" void swdptap_init()
{
    swd_proc.seq_in = SwdRead;
    swd_proc.seq_in_parity = SwdRead_parity;
    swd_proc.seq_out = SwdWrite;
    swd_proc.seq_out_parity = SwdWrite_parity;
}

// EOF

//--
