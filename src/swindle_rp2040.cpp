#include "../swindle/src/platform/rp2040/lnRP2040_pio.h"
#include "lnGPIO.h"
#include "stdint.h"
#define PICO_NO_HARDWARE 1
#include "lnBmpTask.h"

extern "C" void user_init();
void gdb_task(void *parameters);

#define PIN_TO_USE GPIO23 // GPIO16 for zero, GPIO23 for normal size RP2040
void setup()
{
}

/**
 */
void user_init(void)
{
    lnCreateTask(&gdb_task, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
}
#include "swindle_common_rp2040.cpp"
//--
