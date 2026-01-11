#pragma once
#include "bmp_pinout.h"
#include "bmp_swdio_ln.h"
extern uint32_t swd_delay_cnt;
extern SwdPin *rDirection;
extern SwdWaitPin *rSWCLK;
extern SwdDirectionPin *rSWDIO;
extern SwdReset *pReset;
