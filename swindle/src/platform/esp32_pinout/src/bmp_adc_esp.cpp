
/**
 * @file bmp_adc_esp.cpp
 * @brief ESP32 ADC driver for target voltage measurement
 */

#include "adc_cali_schemes.h"
#include "bmp_pinout.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_oneshot.h"
#include "esprit.h"
#include "lnADC.h"
#define CHANNELX (adc_channel_t)(ADC_CHANNEL_0 + PIN_ADC_NRESET_DIV_BY_TWO - 1)
static adc_cali_handle_t cali_handle = NULL;
static adc_oneshot_unit_handle_t adc1_handle;
/*
 *
 *
 */
void gmp_gpio_init_adc()
{
    adc_oneshot_unit_init_cfg_t init_config1 = {
        .unit_id = ADC_UNIT_1,
        .ulp_mode = ADC_ULP_MODE_DISABLE,
    };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&init_config1, &adc1_handle));

    adc_oneshot_chan_cfg_t config = {
        .atten = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, CHANNELX, &config));

    // --- 3. Calibration Initialization (Crucial for mV accuracy) ---
    adc_cali_curve_fitting_config_t cali_config = {
        .unit_id = ADC_UNIT_1,
        .chan = ADC_CHANNEL_0,
        .atten = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    // S3 uses Curve Fitting calibration scheme
    ESP_ERROR_CHECK(adc_cali_create_scheme_curve_fitting(&cali_config, &cali_handle));
}

/*
 *
 *
 */
extern "C" float bmp_get_target_voltage_c()
{
    int adc_raw;
    int voltage_mv;

    // 1. Get raw digital value (0-4095 for 12-bit)
    ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, CHANNELX, &adc_raw));

    // 2. Convert raw value to millivolts using calibration
    ESP_ERROR_CHECK(adc_cali_raw_to_voltage(cali_handle, adc_raw, &voltage_mv));

    return (float)voltage_mv / 1000.f;
}
// EOF
