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

extern void swdioSetAsOutput(bool output);
#if 1
#define SWD_SPEED 100 * 1000UL
#else
#define SWD_SPEED 4 * 1000 * 1000UL
#endif
#define PICO_NO_HARDWARE 1
#define SWD_PIO_TO_USE 1
#define SWD_SM_TO_USE 0
#include "pio_swd.h"
static rpPIO *swdpio;
static rpPIO_SM *xsm;

#define LN_PARITY(x) __builtin_parity(x)
/**
    switch ((int)output)
    {
    case false: // out->in
    {
        pSWDIO.input();
        pSWCLK.wait();
        pSWCLK.clockOn();
        break;
    }
    break;
    case true: // in->out
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
*/

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
    return value;
}
/**
 *
 */
extern "C"
{

    void adiv5_init()
    {
        lnPin pin_swd = pSWDIO.pin();
        lnPin pin_clk = pSWCLK.pin();
        swdpio = new rpPIO(SWD_PIO_TO_USE);
        xsm = swdpio->getSm(SWD_SM_TO_USE);

        lnPinMode(pin_swd, (lnGpioMode)(lnRP_PIO0_MODE + SWD_PIO_TO_USE));
        lnPinMode(pin_clk, (lnGpioMode)(lnRP_PIO0_MODE + SWD_PIO_TO_USE));

        rpPIO_pinConfig pinConfig;
        pinConfig.sets.pinNb = 1;
        pinConfig.sets.startPin = pin_swd;
        pinConfig.outputs.pinNb = 1;
        pinConfig.outputs.startPin = pin_swd;
        pinConfig.inputs.pinNb = 1;
        pinConfig.inputs.startPin = pin_swd;

        xsm->setSpeed(SWD_SPEED);
        xsm->setBitOrder(false, false);
        xsm->setPinDir(pin_swd, true);
        xsm->setPinDir(pin_clk, true);
        xsm->uploadCode(sizeof(swd_program_instructions) / 2, swd_program_instructions, swd_wrap_target, swd_wrap);
        xsm->configure(pinConfig);
        xsm->configureSideSet(pin_clk, 1, 2, true);
        xsm->execute();
        return;
    }
    /**
     * @brief
     *
     * @param RnW
     * @param addr
     * @return uint8_t
     */
    uint8_t LN_FAST_CODE xmake_packet_request(uint8_t RnW, uint16_t addr)
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
        zwrite(28, 0xFFFFFFFF);
        if (idle_cycles)
        {
            zwrite(4, 0);
        }
    }
    /**
     * @brief
     *
     * @param access
     * @param addr
     * @return int
     */
    int LN_FAST_CODE preamble(uint8_t access, const uint32_t addr)
    {
        uint8_t request = xmake_packet_request(access, addr);
        zwrite(8, request);
        return zread(4) >> 1; // one extra bit for turn (lsb)
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
        uint32_t res = preamble(ADIV5_LOW_WRITE, addr);
        if (res != SWDP_ACK_OK)
        {
            Logger("SWD_WRITE: reply not ok 0x%x\n", res);
        }
        zread(1); // one extra bit for turn (lsb)
        zwrite(32, data);
        // par
        zwrite(1, parity);
        //--
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
        uint32_t ret = preamble(ADIV5_LOW_READ, addr);
        if (ret != SWDP_ACK_OK)
        {
            Logger("SWD_REAd: reply not ok 0x%x\n", ret);
        }
        //
        //-----
        ret = zread(32);
        zwrite(8, 0);
        return ret;
    }
}

static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);

void swdioSetAsOutput(bool output);

SwdPin pSWDIO(TSWDIO_PIN);
SwdWaitPin pSWCLK(TSWDCK_PIN); // automatically add delay after toggle
SwdReset pReset(TRESET_PIN);

uint32_t swd_delay_cnt = 1;
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
extern void gmp_gpio_init_adc();
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
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
}

/**
 */
static uint32_t SwdRead(size_t len)
{
    return zread(len);
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    uint32_t val = zread(len);
    uint32_t parity = zread(1);
    bool currentParity = __builtin_parity(val);
    *ret = val;
    return currentParity == parity; // should be equal
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    zwrite(ticks, MS);
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    bool parity = __builtin_parity(MS);
    zwrite(ticks, MS);
    zwrite(1, parity);
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

//--
