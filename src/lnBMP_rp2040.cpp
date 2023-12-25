/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
#include "lnArduino.h"
#define LED LN_SYSTEM_LED
#define LED2 GPIO26

extern "C" void user_init();
/**
 */
void setup()
{
    lnPinMode(LED, lnOUTPUT);
    lnPinMode(LED2, lnOUTPUT);
}
void loop()
{
    Logger("Starting lnBMP Test\n");
    user_init();
    bool onoff = true;
    while (1)
    {
        // Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
        lnDigitalToggle(LED2);
        onoff = !onoff;
    }
}
