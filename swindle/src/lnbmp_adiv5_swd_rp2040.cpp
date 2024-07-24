/*
 *  This is a streamlined version of adiv5_swd.c
 */
// clang-format off
extern "C"
{
#include "exception.h"
#include "general.h"
#include "timing.h"
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
#include "pio_swd.h"
// #include "lnArduino.h"
#include "lnBMP_swdio.h"
#include "ln_rp_pio.h"
// clang-format on
#include "lnRP2040_pio.h"

#if 0
#define dLogger Logger
#else
#define dLogger(...)                                                                                                   \
    {                                                                                                                  \
    }
#endif

#if 1 // USE_RP2040
#define SWD_SPEED 40 * 1000 * 1000UL
#else
#define SWD_SPEED 200 * 1000UL
#endif

#include "lnRP2040_pio.h"
#include "lnbmp_adiv5_common.h"

extern void gmp_gpio_init_adc();
static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);
static void rp2040_swd_pio_init();

#define PICO_NO_HARDWARE 1
#include "pio_swd.h"

static rpPIO *swdpio;
rpPIO_SM *xsm;
uint32_t swd_delay_cnt = 4;
SwdReset pReset(TRESET_PIN);

/**
 *  write size bits over PIO
 */
static void zwrite(uint32_t size, uint32_t value)
{
    uint32_t zsize = ((size - 1) << 1) | 1;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->write(1, &value);
}
/**
 *  read size bits over PIO
 */
static uint32_t zread(uint32_t size)
{
    uint32_t zsize = ((size - 1) << 1) | 0;
    uint32_t value = 0;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->waitRxReady();
    xsm->read(1, &value);
    return value >> (32 - size);
}
/**
 *
 */
extern "C"
{
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
        zwrite(28, 0x0FFFFFFF);
        if (idle_cycles)
            zwrite(4, 0); // not used often, no need to optimize
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
        uint32_t raw = zread(5); // one extra bit for turn (lsb)
        dLogger("RAW ACK_W: %x\n", raw);
        return (raw >> 1) & 7; // ignore 2nd turnaround
    }
    int preamble_r(uint8_t access, const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(access, addr);
        zwrite(8, request);
        uint32_t raw = zread(4);
        dLogger("RAW ACK_R: %x\n", raw);
        return (raw >> 1) & 7;
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
        int parity = lnOddParity(data);
        uint32_t res = preamble_w(ADIV5_LOW_WRITE, addr);
        if (res != SWDP_ACK_OK)
        {
            dLogger("SWD_WRITE: reply not ok 0x%x\n", res);
            return false;
        }
        zwrite(32, data);
        // par
        zwrite(1, parity);
        //-- trailing bits
        zwrite(8, 0);
        return true;
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
            return 0;
        }
        //
        //-----
        ret = zread(32);
        int oparity = zread(1);
        int parity = lnOddParity(ret);
        if (parity != oparity)
        {
            dLogger("SWD:Wrong read parity\n");
        }
        zwrite(8, 0);
        return ret;
    }

    void LN_FAST_CODE adiv5_raw_write(const uint32_t ticks, const uint32_t value)
    {
        zwrite(ticks, value);
    }
    uint32_t LN_FAST_CODE adiv5_raw_read_parity(const uint32_t ticks)
    {
        uint32_t val = zread(32);
        zread(1);
        return val;
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
                dLogger("ReadAck = 0x%x\n", ack);
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
            uint32_t response = zread(32);
            int oparity = zread(1);
            int parity = lnOddParity(response);
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
                ack = preamble_w(ADIV5_LOW_WRITE, addr);
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
            int parity = lnOddParity(value);
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
    pReset.hiZ(); // hi-z by default
    pReset.off(); // hi-z by default

    gmp_gpio_init_adc();
    rp2040_swd_pio_init();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
    pReset.off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    pReset.off(); // hi-z by default
}

/**
 */
void rp2040_swd_pio_init()
{
    lnPin pin_swd = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    swdpio = new rpPIO(LN_SWD_PIO_ENGINE);
    xsm = swdpio->getSm(0);

    lnPinModePIO(pin_swd, LN_SWD_PIO_ENGINE);
    lnPinModePIO(pin_clk, LN_SWD_PIO_ENGINE);

    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_swd;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_swd;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_swd;

    xsm->setSpeed(SWD_SPEED);
    xsm->setBitOrder(true, false);
    xsm->setPinDir(pin_swd, true);
    xsm->setPinDir(pin_clk, true);
    xsm->uploadCode(sizeof(swd_program_instructions) / 2, swd_program_instructions, swd_wrap_target, swd_wrap);
    xsm->configure(pinConfig);
    xsm->configureSideSet(pin_clk, 1, 2, true);
    xsm->execute();
    return;
}
/**
 */
static uint32_t SwdRead(size_t len)
{
    xAssert(0);
    return 0;
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    xAssert(0);
    return false;
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    xAssert(0);
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    xAssert(0);
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
extern "C" void swdptap_init()
{
    swd_proc.seq_in = SwdRead;
    swd_proc.seq_in_parity = SwdRead_parity;
    swd_proc.seq_out = SwdWrite;
    swd_proc.seq_out_parity = SwdWrite_parity;
}

// EOF
