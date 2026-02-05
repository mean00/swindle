/*
  swindle: Gpio driver for Rvswd
  This code is derived from the blackmagic one but has been modified
  to aim at simplicity at the expense of performances (does not matter much though)
  (The compiler may mitigate that by inlining)

 */
/**
 * This is similar to the non rp2040
 *
 */
#include "esprit.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}
#pragma GCC optimize("Ofast")
#include "bmp_pinmode.h"
#include "bmp_rvTap.h"
#include "esprit.h"
#include "bmp_pinout.h"
#include "lnRP2040_pio.h"
#include "ln_rp_pio.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();
extern rpPIO *swdpio;
extern rpPIO_SM *xsm;

#define PIO_BEGIN_INSTRUCTION 2
#define LN_PIO_WRITE_PC (PIO_BEGIN_INSTRUCTION + 0)
#define LN_PIO_STOP_PC (PIO_BEGIN_INSTRUCTION + 1)
#define LN_PIO_START_PC (PIO_BEGIN_INSTRUCTION + 2)
#define LN_PIO_READ_PC (PIO_BEGIN_INSTRUCTION + 3)
//
#define LN_ADDRESS_SHIFT 5
#define rreevv(x) x

#define CMD(x) (rreevv((x)) << (32 - LN_ADDRESS_SHIFT))
#define PARAM(x) (rreevv((x)) << (32 - LN_ADDRESS_SHIFT - LN_ADDRESS_SHIFT))
//
#if 0
#define Debug Logger
#else
#define Debug(...)                                                                                                     \
    {                                                                                                                  \
    }
#endif

static inline void send_command(uint32_t cmd, uint32_t param)
{
    uint32_t zsize = CMD(cmd) | PARAM(param);
    xsm->write(1, &zsize);
}

/**
 * do a falling edge on SWDIO with CLK high (assumed) => start bit
 */
static void rv_start_bit()
{
    Debug("Start bit \n");
    send_command(LN_PIO_START_PC, 0);
}

/**
 *
 * do a rising edge on SWDIO with CLK high (assumed) => stop bit
 */
static void rv_stop_bit()
{
    Debug("Stop bit \n");
    send_command(LN_PIO_STOP_PC, 0);
    xsm->waitTxEmpty();
}
/**
 *
 */
static uint32_t rv_read_nbits(int n)
{
    Debug("Reading %d bits\n", n);
    send_command(LN_PIO_READ_PC, n - 1);
    uint32_t value = 0;
    xsm->waitRxReady();
    xsm->read(1, &value);
    return value;
}

/**
 *
 */
static void rv_write_nbits(int n, uint32_t value)
{
    Debug("Writing %d bits\n", n);
    send_command(LN_PIO_WRITE_PC, n - 1);
    value <<= (32 - n);
    xsm->write(1, &value);
}
/**
 * @brief
 *
 * @return true
 * @return false
 */
bool rv_dm_reset()
{
    xsm->setPinsValue(1);
    // 12*8+4=100 pulses
    for (int i = 0; i < 12; i++)
        rv_write_nbits(8, 0xff);
    rv_write_nbits(4, 0xff);
    return true;
}
/**
 * \fn this is a test function
 */
void rv_write_write(uint32_t value)
{
    Debug("Write Write\n");
    rv_start_bit();
    rv_write_nbits(8, 0xaa55);
    //  rv_write_nbits(8, 0xaa55);
    //  rv_write_nbits(8, 0xaa55);
    //   rv_read_nbits(8);
    rv_stop_bit();
    lnDelayMs(1);
}
#include "rvswd_template.h"
// EOF
