/**
 * @file swindle_w5500.h
 * @brief W5500 Ethernet interface declarations
 */

#pragma once
//
#include "esprit.h"
#include "lnSocketRunner.h"

/** @brief Event group used to signal network-layer events (W5500). */
extern lnFastEventGroup network_eventGroup;

/** @brief TCP port for the GDB server on the W5500. */
#define RUNNER_GDB_PORT 2000

/** @brief TCP port for the RTT server on the W5500. */
#define RUNNER_RTT_PORT 2001

/** @brief Socket slot index used for the main GDB connection. */
#define MAIN_GDB_SLOT 0

/** @brief Socket slot index used for the main RTT connection. */
#define MAIN_RTT_SLOT 6

//
