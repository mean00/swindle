/**
 * @file bmp_rvTap_rp2040.cpp
 * @brief RISC-V DMI transport over esprit GPIO (RP2040)
 */

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
#include "bmp_pinout.h"
#include "bmp_rvTap.h"
#include "esprit.h"
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
 * @brief Emit DMI start bit (SWDIO falling edge while CLK high).
 */
static void rv_start_bit()
{
    Debug("Start bit \n");
    send_command(LN_PIO_START_PC, 0);
}

/**
 * @brief Emit DMI stop bit (SWDIO rising edge while CLK high).
 */
static void rv_stop_bit()
{
    Debug("Stop bit \n");
    send_command(LN_PIO_STOP_PC, 0);
    xsm->waitTxEmpty();
}
/**
 * @brief Read @p n bits MSB-first from SWDIO via PIO.
 * @param n Number of bits to read.
 * @return Sampled bits, MSB-aligned.
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
 * @brief Write @p n bits MSB-first on SWDIO via PIO.
 * @param n     Number of bits to write.
 * @param value Bits to write (MSB-aligned in 32-bit word).
 */
static void rv_write_nbits(int n, uint32_t value)
{
    Debug("Writing %d bits\n", n);
    send_command(LN_PIO_WRITE_PC, n - 1);
    value <<= (32 - n);
    xsm->write(1, &value);
}
/**
 * @brief Reset RISC-V DM via PIO (100 × 1-bit pulses).
 * @return true (always succeeds).
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
 * @brief Test function: write a known pattern over DMI.
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
