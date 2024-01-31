#pragma once

const lnPin _mapping[8] = {
    GPIO24, // 0 TMS_PIN
    GPIO24, // 1 TDI_PIN
    GPIO24, // 2 TDO_PIN
    GPIO24, // 3 TCK_PIN
    GPIO24, // 4 TRACESWO_PIN
    GPIO12, // 5 SWDIO_PIN
    GPIO13, // 6 SWCLK_PIN

    GPIO11, // 7 RST
};

#define PIN_ADC_NRESET_DIV_BY_TWO GPIO25 // this pins is connected to NRST/2
#define PIN_ADC_NRESET_MULTIPLIER 1.     // 2.0 if divided by 2 , 1.0 if not divided

#define LN_USB_INSTANCE 1
#define LN_SERIAL_INSTANCE 1

#define EXTRA_SETUP()                                                                                                  \
    {                                                                                                                  \
    }