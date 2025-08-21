#pragma once

extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }

// clang-format on
static void __inline__ tapOutput();
static void __inline__ tapInput();
//
#include "lnBMP_swdio_ln.h"
#include "lnBMP_swdio_common.h"
//
extern SwdPin *rDirection;

// clang-format on
void __inline__ tapOutput()
{
    rDirection->on();
}
void __inline__ tapInput()
{
    rDirection->off(); // off();
}
//
