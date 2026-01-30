#pragma once
#define SWD_IO_SPEED 10 // programmed max speed of the gpio, 10 Mhz is  more than enough
#include "lnBMP_pins.h"
#ifdef USE_48PIN_PACKAGE
  #include "bmp_pinout_48pins.h"
#else
 #ifdef USE_64PIN_PACKAGE
  #include "bmp_pinout_64pins.h"
 #else
  #error Select 48 or 64 pins package.
 #endif
#endif

extern uint32_t swd_delay_cnt;

#define swait()                                                                                                        \
    {                                                                                                                  \
        for (int lop = swd_delay_cnt; lop > 0; lop--)                                                                  \
            __asm__("nop");                                                                                            \
    }


// EOF
