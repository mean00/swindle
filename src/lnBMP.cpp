#include "lnArduino.h"

#ifdef USE_RP2040
#define LED GPIO25
#define LED2 GPIO25
#else
#define LED PC13
#define LED2 PA8
#endif

extern "C" void user_init();
FILE *const stdout = NULL;
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
    while (1)
    {
        // Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
        lnDigitalToggle(LED2);
    }
}
