#pragma once

// mapping of BMP gpio to the GPIO we use
const lnPin _mapping[9] = {
    GPIO0, // 0 TMS_PIN
    GPIO0, // 1 TDI_PIN
    GPIO0, // 2 TDO_PIN
    GPIO0, // 3 TCK_PIN
    GPIO0, // 4 TRACESWO_PIN

    GPIO18, // 5 SWDIO_PIN
    GPIO17, // 6 SWCLK_PIN

    GPIO2, // 7 RST
    GPIO3, // 8 direction
};

#define PIN_ADC_NRESET_DIV_BY_TWO PA3 // this pins is connected to NRST/2
#define PIN_ADC_NRESET_MULTIPLIER 1.  // 2.0 if divided by 2 , 1.0 if not divided

#define LN_USB_INSTANCE 1
#define LN_SERIAL_INSTANCE 2
#define LN_LOGGER_INSTANCE 2

#define EXTRA_SETUP()                                                                                                  \
    {                                                                                                                  \
    }

extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }
