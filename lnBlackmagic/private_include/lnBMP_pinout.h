#pragma once
#define BM_NB_PINS 7
// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[8]=
{
    PA0, // 0 TMS_PIN
    PA0, // 1 TDI_PIN
    PA0, // 2 TDO_PIN
    PA0, // 3 TCK_PIN
    PA0, // 4 TRACESWO_PIN

    PB4,  // 5 SWDIO_PIN
    PB3,  // 6 SWCLK_PIN

    PB6,  // 7 RST
};

enum lnBMPPins
{
    TTMS_PIN=0,
    TTDI_PIN=1,
    TTDO_PIN=2,
    TTCK_PIN=3,
    TTRACE_PIN=4,
    TSWDIO_PIN=5,
    TSWDCK_PIN=6,
    TRESET_PIN=7,
};
