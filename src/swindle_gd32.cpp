/**
 * @file swindle_gd32.cpp
 * @brief GD32F303 board initialisation and GPIO setup
 */

/*
    On the RP2040 gum stick boards the led is driven by GPIO26, it is NOT open drain
*/
#include "esprit.h"
#include "lnBmpTask.h"
static const lnPin ledPins[] = {LN_SYSTEM_LED, PA8, PB13};
#define NB_LEDS (sizeof(ledPins) / sizeof(lnPin))

extern "C" void user_init();
extern void bmp_gpio_init_once();
extern void gdb_task(void *param);
/**
 */
void setup()
{
    for (int i = 0; i < NB_LEDS; i++)
        lnPinMode(ledPins[i], lnOUTPUT_OPEN_DRAIN);
}
// user_init() — called from loop(), overrides bridge.cpp's empty version
// Initialises SWD pins and creates the socket runner FreeRTOS task.
extern "C" void user_init()
{
    Logger("Starting picoSwindle (W5500)\n");
    bmp_gpio_init_once(); // initialise SWD PIO, reset pin, ADC
    lnCreateTask(&gdb_task, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
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
