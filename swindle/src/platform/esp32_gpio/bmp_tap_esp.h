/*
 *
 */
/**
 * @file bmp_tap_esp.h
 * @brief Globals header for standard-GPIO TAP
 */

#pragma once
#include "bmp_pinout.h"
#include "bmp_swdio_esp.h"
extern uint32_t swd_delay_cnt;
extern SwdWaitPin *rSWCLK;
extern SwdDirectionPin *rSWDIO;
extern SwdReset *pReset;
