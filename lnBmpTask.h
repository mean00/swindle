/**
 * @file lnBmpTask.h
 * @brief FreeRTOS task configuration for the BMP project
 *
 * Holds the stack sizes / priorities of the two big tasks and the decision of
 * *where* their TCB and stack live: in the FreeRTOS heap (xTaskCreate) or in
 * .bss (xTaskCreateStatic, see lnFreeRTOS_pp.h).
 */

#pragma once

#include "lnFreeRTOS_pp.h"

#define TASK_BMP_SERIAL_PRIORITY 4

#define TASK_BMP_GDB_PRIORITY 5

/*
 * Stack sizes, in bytes, measured with `mon stacks` (the per-task high-water
 * mark of the probe's own FreeRTOS kernel) on the GD32F303 probe running the
 * worst case the debugger has: `rvswdp_scan` + `attach` + `load` of a 200 KB
 * CH32V3x image + `compare-sections` + `mon riscv_benchmark` +
 * `mon riscv_memory_benchmark` + UART traffic on the bridge.
 *
 *   gdbTask   : 6144 B stack, 3408 B still free  -> 2736 B peak  -> 2 -> 4 KiB
 *   usbserial : 6144 B stack, 5916 B still free  ->  228 B peak  -> 2 KiB
 *
 * Both keep a margin on top of the measured peak: +1360 B (50%) for gdbTask,
 * which is the task that runs the whole GDB/flash/transport stack, and +1820 B
 * (8x) for the bridge loop.  Before this the two numbers were guesses (6 KiB
 * each, `malloc_used.md` §8.1 S5); they cost 8 KiB of the FreeRTOS pool between
 * them, five sixths of it for `usbserial`.
 */
#define TASK_BMP_SERIAL_STACK_SIZE (2 * 1024) // in bytes
#define TASK_BMP_GDB_STACK_SIZE (4 * 1024)    // in bytes

/*
 * Static or heap TCB + stack for those two tasks.
 *
 * A static task costs its stack + 112 B of .bss, and .bss and ucHeap are the
 * same RAM: the heap has to give those bytes back (LN_MCU_STATIC_RAM grows by
 * the same amount, LN_FREERTOS_HEAP_SIZE = RAM - STATIC_RAM - 1 shrinks,
 * malloc_used.md §6 D5).  Once the stacks are right-sized that trade is a win:
 * 2 x (6144 + 112) = 12512 B leave the pool, the budget grows by 7 KiB, so the
 * bytes left for *allocation* go 5920 -> 11264 B on both the 39 KiB GD32F303
 * and the 64 KiB CH32V307, and a fragmented heap can no longer eat a stack.
 *
 * The 20 KiB bluepill part cannot afford it (17 + 7 KiB of static + a pool is
 * all it has), so it keeps both tasks on the heap: the CMake side sets
 * LN_BIG_TASKS_STATIC_STACK for the boards that can (top-level CMakeLists.txt).
 * entryTask and usbd (3 KiB together) are static on every board, see
 * malloc_used.md §8.1 S5.
 */
#ifndef LN_BIG_TASKS_STATIC_STACK
#define LN_BIG_TASKS_STATIC_STACK 0
#endif

/* Storage of one big task, declared at file scope, e.g.
 *     LN_BIG_TASK_STORAGE(gdbTask, TASK_BMP_GDB_STACK_SIZE);
 * and created with LN_CREATE_BIG_TASK() (see below).  `tag` names the two
 * objects, `name` is the FreeRTOS task name.  Both macros fall back to plain
 * xTaskCreate() when the stack stays in the heap. */
#if LN_BIG_TASKS_STATIC_STACK
#define LN_BIG_TASK_STORAGE(tag, stackSizeInBytes)                                                                     \
    static StaticTask_t tag##_tcb;                                                                                     \
    static StackType_t tag##_stack[LN_TASK_STACK_WORDS(stackSizeInBytes)]

#define LN_CREATE_BIG_TASK(entry, tag, name, stackSizeInBytes, param, priority)                                        \
    lnCreateTaskStatic(entry, name, stackSizeInBytes, param, priority, tag##_stack, &tag##_tcb)
#else
#define LN_BIG_TASK_STORAGE(tag, stackSizeInBytes)
#define LN_CREATE_BIG_TASK(entry, tag, name, stackSizeInBytes, param, priority)                                        \
    lnCreateTask(entry, name, stackSizeInBytes, param, priority)
#endif
