#pragma once

#define TASK_BMP_SERIAL_PRIORITY 4

#define TASK_BMP_GDB_PRIORITY 5

#if 1                                      // small footprint, min heap = 21560
#define TASK_BMP_SERIAL_STACK_SIZE (5*1024)  // in bytes
#define TASK_BMP_GDB_STACK_SIZE (5*1024) // in bytes
#else                                      // large fooprint min heap = 12584
#define TASK_BMP_SERIAL_STACK_SIZE (5000)  // in bytes
#define TASK_BMP_GDB_STACK_SIZE (8072)     // in bytes
#endif
