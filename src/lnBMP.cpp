#include "lnArduino.h"

#define LED LN_SYSTEM_LED
#define LED2 PA8

extern "C" void user_init();

extern "C" void rnLoop();

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
    rnLoop();
    while (1)
    {
        // Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
        lnDigitalToggle(LED2);
    }
}
