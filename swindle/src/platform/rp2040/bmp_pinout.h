/**
 * @file bmp_pinout.h
 * @brief RP2040 pin mapping header include
 */

#pragma once

#include "lnGPIO.h"
#ifdef USE_RP_CARRIER
#include "bmp_pinout_rp2040_carrier.h"
#else
#include "bmp_pinout_rp2040.h"
#endif
