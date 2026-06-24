#include "ws.h"
#include "xsin_table.h"
#include "../swindle/src/platform/rp2040/lnRP2040_pio.h"
#include "ln_rp_pio.h"
static uint32_t colorShift = 0;
void ln_set_connected_state(bool state)
{
    if (state)
        colorShift = 16;
    else
        colorShift = 0;
}
// loop() — called repeatedly by rp.cpp's initTask
// Runs the WS2812 LED breathing animation (same as USB builds).
void loop()
{
    Logger("Starting picoSwindle\n");
    user_init();
    while (0)
    {
        lnDelayMs(100);
    }

    lnPin pin = PIN_TO_USE;
    rpPIO xpio(LN_WS2812_PIO_ENGINE);
    rpPIO_SM *xsm = xpio.getSm(0);
    lnPinModePIO(pin, LN_WS2812_PIO_ENGINE);
    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin;
    xsm->setSpeed(WS_SPEED);
    xsm->setBitOrder(false, true);
    xsm->uploadCode(sizeof(ws_program_instructions) / 2, ws_program_instructions, ws_wrap_target, ws_wrap);
    xsm->configure(pinConfig);
    xsm->setPinDir(pin, true);
    xsm->execute();

    int pix = 0;
    int inc = STEP;
    while (1)
    {
        uint32_t wait = (xsin[pix] >> 4) << colorShift;
        xsm->write(1, &wait);
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
                inc = -inc;
        }
        lnDelayMs(6);
    }
}
