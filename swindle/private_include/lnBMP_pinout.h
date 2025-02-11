#pragma once

#define BM_NB_PINS 7

// Logical identifiers..
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
    TDIRECTION_PIN = 8,
};

#ifdef USE_RP2040
#ifdef USE_RP_CARRIER
#include "lnBMP_pinout_rp_carrier.h"
#else
#include "lnBMP_pinout_rp2040.h"
#endif
#else
#ifdef USE_GD32F303
#include "lnBMP_pinout_ln_gd32f303.h"
#else
#include "lnBMP_pinout_ln_ch32v3xx.h"
#endif
#endif
// EOF
