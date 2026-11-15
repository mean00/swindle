/**
 * @file gdb_server_common.cpp
 * @brief Probe back end for a USB probe: the BMP core behind the GDB server.
 *
 * The session itself is served by the Rust runner
 * (swindle/rs/rs_swindle/src/hosted/qt_runner.rs, `GdbHandler`), which calls the
 * BMP core; the packet decoders are C++ and stay here.
 */

#include "hosted_constants.h"
#include "hosted_probe.h"

#include "esp_socket_rust.h"

/**
 * @brief Build the GDB runner for a USB probe.
 *
 * A null probe endpoint selects the GDB-core back end (the network relay passes
 * one, see bmp_qttcp.cpp).
 */
void *hosted_create_runner(lnFastEventGroup &eventGroup)
{
    return hosted_runner_new(&eventGroup, DEFAULT_GDB_TCP_PORT, nullptr, 0);
}
