/**
 * @file bmp_tap_fesp.h
 * @brief ESP32 fast-GPIO TAP globals.
 */
#pragma once
#include "bmp_pinout.h"
#include "bmp_swdio_fesp.h"
#include "stdint.h"
extern uint32_t swd_delay_cnt;
extern SwdWaitPin *rSWCLK;
extern SwdDirectionPin *rSWDIO;
extern SwdReset *pReset;
