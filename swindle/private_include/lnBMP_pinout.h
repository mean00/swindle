#pragma once

#define BM_NB_PINS 7
#define SWD_IO_SPEED 10 // 10mhz is more than enough (?)

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

#if defined(LN_SWINDLE_AS_EXTERNAL)
#include "lnBMP_pinout_external.h"
#else
#if defined(USE_RP2040) || defined(USE_RP2350)
#ifdef USE_RP_CARRIER
#include "lnBMP_pinout_rp_carrier.h"
#else
#include "lnBMP_pinout_rp2040.h"
#endif
#else
#ifdef USE_48PIN_PACKAGE
#include "lnBMP_pinout_bp_48pins.h"
#elif defined(USE_64PIN_PACKAGE)
#include "lnBMP_pinout_bp_64pins.h"
#else
#error "Fatal error : select 48 pins or 64 pins package"
#endif
#endif
#endif
// EOF
