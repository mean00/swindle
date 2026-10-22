/**
 * @file swindle_gd32.cpp
 * @brief GD32F303 / CH32V3x board initialisation
 *        The led activity (blinking leds) lives in swindle/rs/rs_swindle_leds
 */

#include "esprit.h"
#include "lnBmpTask.h"

extern "C" void user_init();
extern "C" void swindle_leds_run(); // rs_swindle_leds, never returns
extern void bmp_gpio_init_once();
extern void gdb_task(void *param);

// gdbTask TCB + stack: static when the board can spare the 6 KiB (lnBmpTask.h)
LN_BIG_TASK_STORAGE(gdbTask, TASK_BMP_GDB_STACK_SIZE);
/**
 */
void setup()
{
}
// user_init() — called from loop(), overrides bridge.cpp's empty version
// Initialises SWD pins and creates the socket runner FreeRTOS task.
extern "C" void user_init()
{
    Logger("Starting picoSwindle (W5500)\n");
    bmp_gpio_init_once(); // initialise SWD PIO, reset pin, ADC
    LN_CREATE_BIG_TASK(&gdb_task, gdbTask, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
}

void loop()
{
    Logger("Starting Swindle\n");
    user_init();
    swindle_leds_run();
}
