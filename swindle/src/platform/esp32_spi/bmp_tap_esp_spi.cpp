/*
 *
 */
#include "esprit.h"
#include "bmp_pinout.h"
#include "bmp_swdio_esp_spi.h"
#include "lnCpuID.h"
#include "math.h"
#include "bmp_tap_esp_spi.h"
//
#include "driver/spi_master.h"
#include "hal/gpio_types.h"

spi_device_handle_t swd_spi;
extern void gmp_gpio_init_adc();

uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
SwdReset *pReset;
/**
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    return;
}
/**
 * @brief
 *
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    swd_frequency = fq;
}
/**
 * @brief
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return 1;
}
/**

*/
void bmp_gpio_init_once()
{
    pReset = new SwdReset(TRESET_PIN); // automatically add delay after toggle
    spi_bus_config_t buscfg = {
        .mosi_io_num = _mapping[TSWDIO_PIN], // SWDIO
        .miso_io_num = -1,                   // Not used (using half-duplex)
        .sclk_io_num = _mapping[TSWDCK_PIN], // SWCLK
        .quadwp_io_num = -1,
        .quadhd_io_num = -1,
        .max_transfer_sz = 4096,
    };

    spi_device_interface_config_t devcfg = {
        .mode = 0,                       // CPOL=0, CPHA=0
        .clock_speed_hz = swd_frequency, // 1 MHz
        .spics_io_num = -1,              // No CS pin
        .flags = SPI_DEVICE_HALFDUPLEX | SPI_DEVICE_BIT_LSBFIRST,
        .queue_size = 7,
    };

    spi_bus_initialize(SPI2_HOST, &buscfg, SPI_DMA_CH_AUTO);
    spi_bus_add_device(SPI2_HOST, &devcfg, &swd_spi);
    //
    //
    pReset->setup();
    pReset->off(); // hi-z by default
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
//
