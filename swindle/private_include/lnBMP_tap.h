#pragma once
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"

extern uint32_t swd_delay_cnt;
extern SwdPin *rDirection;
extern SwdWaitPin *rSWCLK;
extern SwdDirectionPin *rSWDIO;
extern SwdReset pReset;
