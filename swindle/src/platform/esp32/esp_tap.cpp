/**
 * @file
 * @brief [TODO:description]
 */

// TAP
//
#include "esprit.h"
//
#include "bmp_pinmode.h"
#include "driver/spi_master.h"
#include "driver/gpio.h"
//
#include "lnBMP_pinout.h"
spi_device_handle_t esp_spi_handle = NULL;
uint32_t esp_spi_speed = 10000;

/*
 *
 *
 */
extern "C" void swdptap_init()
{
}
/*
 *
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    Logger("Unsupported call in PIO mode\n");
    return;
}
/**
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    Logger("Unsupported call in PIO mode\n");
    return 0;
}
/*
 *
 *
 */
void bmp_gpio_init_once()
{

    spi_bus_config_t buscfg = {};
    buscfg.mosi_io_num = _mapping[TSWDIO_PIN];
    buscfg.miso_io_num = _mapping[TSWDIO_PIN2];
    buscfg.sclk_io_num = _mapping[TSWDCK_PIN];
    buscfg.quadwp_io_num = -1;
    buscfg.quadhd_io_num = -1;
    buscfg.max_transfer_sz = 64; // We only need small bursts

    esp_err_t ret = spi_bus_initialize(SPI2_HOST, &buscfg, SPI_DMA_CH_AUTO);
    ESP_ERROR_CHECK(ret);

    spi_device_interface_config_t devcfg = {};
    devcfg.clock_speed_hz = esp_spi_speed;
    devcfg.mode = 0;
    devcfg.spics_io_num = -1;
    devcfg.queue_size = 7;
    devcfg.flags = 0;
    ret = spi_bus_add_device(SPI2_HOST, &devcfg, &esp_spi_handle);
    ESP_ERROR_CHECK(ret);
    // make the default state 1
    gpio_set_pull_mode((gpio_num_t)_mapping[TSWDIO_PIN], GPIO_PULLUP_ONLY);
}

/*
 *
 *
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    esp_spi_speed = fq;
}
/*
 *
 */
extern "C" uint32_t bmp_get_frequency_c()
{
    return esp_spi_speed;
}
/*
 *
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}

// EOF
