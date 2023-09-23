#pragma once
#define BM_NB_PINS 7
//#define GPIO25 1
// mapping of BMP gpio to the GPIO we use

#ifdef USE_STLINK_PINOUT
const lnPin _mapping[8] = {
    GPIO25, // 0 TMS_PIN
    GPIO25, // 1 TDI_PIN
    GPIO25, // 2 TDO_PIN
    GPIO25, // 3 TCK_PIN
    GPIO25, // 4 TRACESWO_PIN

    GPIO25, // 5 SWDIO_PIN
    GPIO25, // 6 SWCLK_PIN

    GPIO25, // 7 RST
};
#else

const lnPin _mapping[8] = {
    GPIO25,        // 0 TMS_PIN
    GPIO25,        // 1 TDI_PIN
    GPIO25,        // 2 TDO_PIN
    GPIO25,        // 3 TCK_PIN
    GPIO25,        // 4 TRACESWO_PIN
    GPIO25, // 5 SWDIO_PIN
    GPIO25,        // 6 SWCLK_PIN

    GPIO25, // 7 RST
};

#endif
enum lnBMPPins
{
    TTMS_PIN = 0,
    TTDI_PIN = 1,
    TTDO_PIN = 2,
    TTCK_PIN = 3,
    TTRACE_PIN = 4,
    TSWDIO_PIN = 5,
    TSWDCK_PIN = 6,
    TRESET_PIN = 7,
};

#define PIN_ADC_NRESET_DIV_BY_TWO GPIO25 // this pins is connected to NRST/2
#define PIN_ADC_NRESET_MULTIPLIER 1.  // 2.0 if divided by 2 , 1.0 if not divided
