#include "lnArduino.h"

#define LED LN_SYSTEM_LED

extern "C" void user_init();

/**
 */
void setup()
{
    lnPinMode(LED, lnOUTPUT);
}
void loop()
{
    Logger("Starting lnBMP Test\n");
    user_init();
    while (1)
    {
        Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
    }
}
