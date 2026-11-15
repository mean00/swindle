/**
 * @file hosted_no_rpc.cpp
 * @brief Probe RPC transport placeholders for the network-probe build.
 *
 * `swindle_hosted_net` relays GDB over TCP and never speaks the probe RPC, but
 * the Rust BMP crate — linked in both builds for `target_list_free()` and the
 * GDB stub helpers — still references the four transport entry points. They are
 * unreachable from this build, so they report that instead of silently
 * pretending to work.
 */

#include <cstdint>
#include <cstdio>

#include "hosted_constants.h"

extern "C"
{

/** No serial RPC in this build. */
int platform_buffer_read(uint8_t *data, int maxsize)
{
    (void)data;
    (void)maxsize;
    fprintf(stderr, "[Net] platform_buffer_read: no probe RPC in the network build\n");
    return SERIAL_READ_ERROR_IO;
}

/** No serial RPC in this build. */
int platform_buffer_write(const uint8_t *data, int size)
{
    (void)data;
    (void)size;
    fprintf(stderr, "[Net] platform_buffer_write: no probe RPC in the network build\n");
    return SERIAL_READ_ERROR_IO;
}

/** No serial RPC in this build. */
int platform_buffer_write_buffered(const uint8_t *data, int size)
{
    return platform_buffer_write(data, size);
}

/** No serial RPC in this build. */
void platform_write_flush()
{
}
}
