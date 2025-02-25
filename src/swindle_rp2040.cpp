// -------------------------------------------------- //
// This file is autogenerated by pioasm; do not edit! //
// -------------------------------------------------- //
#include "../swindle/private_include/lnRP2040_pio.h"
#include "lnGPIO.h"
#include "ln_rp_pio.h"
#include "stdint.h"
#define PICO_NO_HARDWARE 1
#include "ws.h"

extern "C" void user_init();

/**
 * @brief
 *
 */
const uint8_t xsin[181] = {
    0,   0,   0,   0,   0,   0,   1,   1,   1,   2,   2,   2,   3,   3,   4,   4,   5,   6,   6,   7,   8,   9,   9,
    10,  11,  12,  13,  14,  15,  16,  17,  18,  19,  21,  22,  23,  24,  26,  27,  29,  30,  31,  33,  34,  36,  37,
    39,  41,  42,  44,  46,  47,  49,  51,  53,  55,  56,  58,  60,  62,  64,  66,  68,  70,  72,  74,  76,  78,  80,
    82,  84,  86,  88,  91,  93,  95,  97,  99,  101, 104, 106, 108, 110, 112, 115, 117, 119, 121, 124, 126, 128, 129,
    131, 134, 136, 138, 140, 143, 145, 147, 149, 151, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175, 177, 179,
    181, 183, 185, 187, 189, 191, 193, 195, 197, 199, 200, 202, 204, 206, 208, 209, 211, 213, 214, 216, 218, 219, 221,
    222, 224, 225, 226, 228, 229, 231, 232, 233, 234, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 246, 247,
    248, 249, 249, 250, 251, 251, 252, 252, 253, 253, 253, 254, 254, 254, 255, 255, 255, 255, 255, 255,
};

#define WS_SPEED (800000 * 10)
#ifdef USE_RP_ZERO
#define PIN_TO_USE GPIO16 // GPIO16 for zero, GPIO23 for normal size RP2040
#else
#define PIN_TO_USE GPIO23 // GPIO16 for zero, GPIO23 for normal size RP2040
#endif

#define STEP 1
void setup()
{
}
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
        uint32_t wait = xsin[pix] << 16;
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
            {
                inc = -inc;
            }
        }
        lnDelayMs(6);
    }
}

//--
