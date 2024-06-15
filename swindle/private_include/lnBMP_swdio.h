#pragma once

#define SWD_IO_SPEED 10 // 10mhz is more than enough (?)
extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; --lop > 0;)                                                                      \
            __asm__("nop");                                                                                            \
    }

#include "lnBMP_swdio_ln.h"
#include "lnBMP_swdio_common.h"
