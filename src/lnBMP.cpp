/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
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
#ifdef USE_RP2040
    lnPinMode(LED, lnOUTPUT);
    lnPinMode(LED2, lnOUTPUT);
#else
    lnPinMode(LED, lnOUTPUT_OPEN_DRAIN);
    lnPinMode(LED2, lnOUTPUT_OPEN_DRAIN);
#endif    
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
#ifdef USE_RP2040        
        lnDigitalToggle(LED);
        lnDigitalToggle(LED2);
#else
        lnOpenDrainClose(LED, onoff);
        lnOpenDrainClose(LED2, !onoff);
#endif
        onoff=!onoff;
    }
}
