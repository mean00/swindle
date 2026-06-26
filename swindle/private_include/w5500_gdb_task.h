/*
 * @file    w5500_gdb_task.h
 * @brief   Shared GDB task helper for W5500-based boards.
 *
 * Provides the shared declarations needed by W5500 board source files:
 * - Blackmagic headers (via bmp_net_gdb.h / bmp_net_rtt.h)
 * - W5500 low-level driver (lowlevel_w5500.h)
 * - socketRunner classes
 * - process_sockets() helper
 * - Global runnerGdb / runnerRtt pointers
 * - NetCb_c callback
 *
 * Usage (in board-specific source file):
 * @code
 *   #include "w5500_gdb_task.h"
 *   // ... define W5500 pin mapping, MAC address ...
 *   void setup() {
 *       W5500LowLevel::init(0, &w5500Pins);
 *       W5500LowLevel::setMac(mac);
 *       W5500LowLevel::start(NetCb_c, NULL);
 *   }
 *   void loop() {
 *       user_init();
 *       // ... event loop ...
 *   }
 * @endcode
 *
 * @copyright (C) 2025
 * @license  See license file
 */

#pragma once

#include "esprit.h"

// DEBUGME is used by bmp_net_gdb.h / bmp_net_rtt.h for debug logging
#ifndef DEBUGME
#define DEBUGME(...) Logger(__VA_ARGS__)
#endif

// Blackmagic headers (required by socket runner classes)
extern "C"
{
#include "exception.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "target.h"
#include "version.h"
}

#define RUNNER_GDB_PORT 2000
#define RUNNER_RTT_PORT 2001

#include "lowlevel_w5500.h"
#include "lnSocketRunner.h"
#include "bmp_net_gdb.h"
#include "bmp_net_rtt.h"

extern "C" void pins_init();
extern void serialInit();
extern "C" void bmp_io_begin_session();
extern "C" void bmp_io_end_session();

/**
 * @brief Dummy stub required by the BMP GDB layer.
 */
extern "C" int gdb_network_init(void)
{
    return 0;
}

/**
 * @brief Dummy stub — no FreeRTOS init needed for W5500.
 */
void initFreeRTOS()
{
}

/**
 * @brief Dummy stub — no GDB interface init needed for W5500.
 */
void gdb_if_init()
{
}

#define MAIN_GDB_SLOT 0
#define MAIN_RTT_SLOT 6

/** @brief Event group used to signal socketRunner events. */
lnFastEventGroup network_eventGroup;

/**
 * @brief Callback from W5500LowLevel (or lnLWIP-compatible) network events.
 *
 * Translates LwipReady/LwipDown into socketRunner::Up/Down events.
 *
 * @param evt  Network event.
 * @param arg  Opaque user argument (unused).
 */
static void NetCb_c(lnLwipEvent evt, void *arg)
{
    (void)arg;
    socketRunner::RunnerEvent revt;
    switch (evt)
    {
    case LwipDown:
        revt = socketRunner::Down;
        break;
    case LwipReady:
        revt = socketRunner::Up;
        break;
    default:
        xAssert(0);
        break;
    }
    network_eventGroup.setEvents(revt);
}

/**
 * @brief Process pending events for a single socket runner.
 *
 * @param runner  Pointer to the socket runner instance.
 * @param global  Global events (Up/Down).
 * @param locl    Local events (per-slot bitmask).
 */
static void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl)
{
    uint32_t limited = (locl >> runner->shift()) & socketRunner::Mask;
    runner->process_events(limited | global);
}

/**
 * @brief Debug serial output stub.
 *
 * @param data  Data to output.
 * @param len   Length of data.
 */
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data);
}
// EOF