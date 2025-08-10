/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
#include "esprit.h"
static const lnPin ledPins[] = {LN_SYSTEM_LED, PA8, PB13};
#define NB_LEDS (sizeof(ledPins) / sizeof(lnPin))

#include "xsin_table.h"
#include "lnWS2812B_timer.h"

#define PIN_WS2812 PB4

extern "C" void user_init();
/**
 */
void setup()
{
    lnRemapTimerPin(2, PartialRemap); // remap timer 2 to pb4/pb5
    lnPinMode(PIN_WS2812, lnPWM);
}
void loop()
{
    Logger("Starting Swindle\n");
    user_init();
    WS2812B_timer *ws = new WS2812B_timer(1, PIN_WS2812);
    ws->begin();
    bool onoff = true;
    int pix = 0;
    int inc = STEP;
    while (1)
    {
        uint32_t wait = xsin[pix];
        ws->setLedColor(0, wait, 0, 0);
        ws->update();
        pix += inc;
        if (inc == STEP)
        {
            if (pix > 180 - STEP)
            {
                inc = -inc;
            }
        }
        else
        {
            if (pix == 0)
            {
                inc = -inc;
            }
        }
        lnDelayMs(6);
    }
}
//
