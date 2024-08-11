#pragma once

#define SWD_IO_SPEED 10 // 10mhz is more than enough (?)
extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }
// clang-format off
#include "lnBMP_swdio_ln.h"
#include "lnBMP_swdio_common.h"
// clang-format on
