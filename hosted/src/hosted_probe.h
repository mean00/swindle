/**
 * @file hosted_probe.h
 * @brief Seam between the shared hosted applications and the probe backend.
 *
 * `swindle_hosted` (USB CDC) and `swindle_hosted_net` (TCP) are the same
 * program apart from what sits behind the local GDB server:
 *
 *  - the USB backend forwards the GDB stream to the BMP core, which talks to
 *    the probe over the serial RPC (`bmp_qtserial.cpp` + `gdb_server_common.cpp`);
 *  - the network backend relays the GDB stream verbatim to the probe's own TCP
 *    server (`bmp_qttcp.cpp`).
 *
 * Each executable links exactly one backend, so these three hooks are the only
 * per-backend code `main.cpp` / `main_qt.cpp` see.
 */

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "lnFreeRTOS_pp.h"

/** Creates (does not start) the runner that serves the GDB client. */
using HostedRunnerFactory = void *(*)(lnFastEventGroup &eventGroup);

/** Runner factory of this build's back end. */
void *hosted_create_runner(lnFastEventGroup &eventGroup);

/**
 * @brief Parse this back end's own command line options.
 * @param argc  Application argument count.
 * @param argv  Application arguments.
 * @return false when the arguments are unusable (the caller prints the usage).
 */
bool hosted_probe_parse_args(int argc, char **argv);

/** Print this back end's command line options. */
void hosted_probe_usage();

/**
 * @brief Print what this back end is about to talk to.
 * @param raw_dmi  The `--raw-dmi` selection (only meaningful for the RPC path).
 */
void hosted_probe_print_banner(bool raw_dmi);
