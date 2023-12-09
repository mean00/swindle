#pragma once

#define SWD_IO_SPEED 10 // 10mhz is more than enough (?)

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; --lop > 0;)                                                                      \
            __asm__("nop");                                                                                            \
    }

#ifdef USE_RP2040
#include "lnBMP_swdio_rp2040.h"
#else
#include "lnBMP_swdio_ln.h"
#endif

#include "lnBMP_swdio_common.h"
