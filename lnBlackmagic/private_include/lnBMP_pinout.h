#pragma once
#define BM_NB_PINS 7
// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[8]=
{
    PA10, // 0 TMS_PIN
    PA10, // 1 TDI_PIN
    PA10, // 2 TDO_PIN
    PA10, // 3 TCK_PIN
    PA10, // 4 TRACESWO_PIN
    PB4,  // 5 SWDIO_PIN
    PB3,  // 6 SWCLK_PIN
    PB6,  // 7 RST
};
