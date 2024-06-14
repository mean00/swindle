/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
#include "lnArduino.h"
#include "lnWS2812_rp_single_pio.h"

// "Big" board = GPIO23
// "Zero" board = GPIO16
#define LED_WS2812 GPIO16 // GPIO16 for zero, GPIO23 for normal size RP2040
// #define LED_WS2812 GPIO23 // GPIO16 for zero, GPIO23 for normal size RP2040
extern "C" void user_init();

#define STEP 8
static int pix = 0, inc = STEP;
static WS2812_rp2040_pio_single *ws;

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

/**
 */
void setup()
{
    ws = new WS2812_rp2040_pio_single(1, 0, LED_WS2812);
}
/**
 * @brief
 *
 */
void loop()
{
    Logger("Starting Swindle\n");
    user_init();
    bool onoff = true;
    while (1)
    {
        ws->setColor(0, 0, xsin[pix]);
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
        lnDelayMs(50);
    }
}
