/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
#include "esprit.h"
static const lnPin ledPins[] = {LN_SYSTEM_LED, PA8, PB13};
#define NB_LEDS (sizeof(ledPins) / sizeof(lnPin))

extern "C" void user_init();
/**
 */
void setup()
{
    for (int i = 0; i < NB_LEDS; i++)
        lnPinMode(ledPins[i], lnOUTPUT_OPEN_DRAIN);
}
void loop()
{
    Logger("Starting Swindle\n");
    user_init();
    bool onoff = true;
    while (1)
    {
        // Logger("*\n");
        lnDelayMs(1000);
        for (int i = 0; i < NB_LEDS; i++)
            lnOpenDrainClose(ledPins[i], onoff);
        onoff = !onoff;
    }
}
