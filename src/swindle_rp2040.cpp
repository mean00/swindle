/**
 * @file swindle_rp2040.cpp
 * @brief RP2040 board initialisation entry point
 */

#include "../swindle/src/platform/rp2040/lnRP2040_pio.h"
#include "lnGPIO.h"
#include "stdint.h"
#define PICO_NO_HARDWARE 1
#include "lnBmpTask.h"

extern "C" void user_init();
void gdb_task(void *parameters);

#define PIN_TO_USE GPIO23 // GPIO16 for zero, GPIO23 for normal size RP2040
// gdbTask TCB + stack: static when the board can spare the 6 KiB (lnBmpTask.h)
LN_BIG_TASK_STORAGE(gdbTask, TASK_BMP_GDB_STACK_SIZE);
void setup()
{
}

/**
 */
void user_init(void)
{
    LN_CREATE_BIG_TASK(&gdb_task, gdbTask, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
}
#include "swindle_common_rp2040.cpp"
//--
