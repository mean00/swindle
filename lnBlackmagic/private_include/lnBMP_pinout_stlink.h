#pragma once
const lnPin _mapping[8] = {
    PA0, // 0 TMS_PIN
    PA0, // 1 TDI_PIN
    PA0, // 2 TDO_PIN
    PA0, // 3 TCK_PIN
    PA0, // 4 TRACESWO_PIN

    PA13, // 5 SWDIO_PIN
    PA14, // 6 SWCLK_PIN

    PB4, // 7 RST
};

#define PIN_ADC_NRESET_DIV_BY_TWO PA3 // this pins is connected to NRST/2
#define PIN_ADC_NRESET_MULTIPLIER 1.  // 2.0 if divided by 2 , 1.0 if not divided

#define LN_USB_INSTANCE 1
#define LN_SERIAL_INSTANCE 0

#define EXTRA_SETUP()                                                                                                  \
    {                                                                                                                  \
        extern void lnExtiNoSWD();                                                                                     \
        lnPinMode(PA9, lnOUTPUT);                                                                                      \
        lnPinMode(PA10, lnOUTPUT);                                                                                     \
        lnPinMode(PB6, lnALTERNATE_PP);                                                                                \
        lnPinMode(PB7, lnALTERNATE_PP);                                                                                \
        lnExtiNoSWD();                                                                                                 \
    }
