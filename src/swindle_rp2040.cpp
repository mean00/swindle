/**
 * @file swindle_rp2040.cpp
 * @brief RP2040 board initialisation entry point
 */

#include "esprit.h"
//
#include "lnGPIO.h"
#include "stdint.h"
#define PICO_NO_HARDWARE 1
#include "lnBmpTask.h"
#ifdef USE_RTT
#include "LN_RTT.h"
extern void rttLoggerFunction(int n, const char *data);
#endif

extern "C" void user_init();
void gdb_task(void *parameters);

// gdbTask TCB + stack: static when the board can spare the 6 KiB (lnBmpTask.h)
LN_BIG_TASK_STORAGE(gdbTask, TASK_BMP_GDB_STACK_SIZE);
void setup()
{
#ifdef USE_RTT
    setLogger(rttLoggerFunction);
    LN_RTT_Init();
#endif
}

/**
 */
void user_init(void)
{
    LN_CREATE_BIG_TASK(&gdb_task, gdbTask, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
}
#include "swindle_common_rp2040.cpp"
//--
