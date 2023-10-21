#include "lnArduino.h"

#define LED LN_SYSTEM_LED
#ifdef USE_RP2040
    #define LED2 GPIO26
#else
    #define LED2 PA8
#endif

extern "C" void user_init();
/**
 */
void setup()
{
    lnPinMode(LED, lnOUTPUT_OPEN_DRAIN);
    lnPinMode(LED2, lnOUTPUT_OPEN_DRAIN);
}
void loop()
{
    Logger("Starting lnBMP Test\n");
    user_init();
    bool onoff=true;
    while (1)
    {
        // Logger("*\n");
        delay(1000);
        lnOpenDrainClose(LED, onoff);
        lnOpenDrainClose(LED2, !onoff);
        onoff=!onoff;
    }
}
