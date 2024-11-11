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
};

#ifdef USE_RP2040
#ifdef USE_RP_CARRIER
#include "lnBMP_pinout_rp_carrier.h"
#else
#include "lnBMP_pinout_rp2040.h"
#endif
#elif defined(USE_STLINK_PINOUT)
#include "lnBMP_pinout_stlink.h"
#else
#include "lnBMP_pinout_ln.h"
#endif
// EOF
