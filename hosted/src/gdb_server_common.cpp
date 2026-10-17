/**
 * @file gdb_server_common.cpp
 * @brief Common implementations shared between CLI and GUI hosted applications.
 */

#include "gdb_server_thread.h"

// Intercept rngdb_send_data_c from bmp_net_gdb.h to decode replies on the fly
#define rngdb_send_data_c orig_rngdb_send_data_c
#include "../../swindle/src/net/bmp_net_gdb.h"
#undef rngdb_send_data_c

extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    gdb_decode_outgoing(ptr, sz);
    orig_rngdb_send_data_c(sz, ptr);
}

static void hosted_rngdbstub_run_intercept(uint32_t len, const uint8_t *data)
{
    gdb_decode_incoming(data, len);
    rngdbstub_run(len, data);
}

#define DEBUGME(...) do {} while (0)
// Redirect socketRunnerGdb::process_incoming_data() to the decoding interceptor
#define rngdbstub_run hosted_rngdbstub_run_intercept
#include "bmp_net_gdb.cpp"
#undef rngdbstub_run

static constexpr size_t TEMP_BUFFER_WORDS = 1024;
static uint32_t s_temp_buffer[TEMP_BUFFER_WORDS];

extern "C" uint8_t *get_temp_buffer_c(uint32_t size)
{
    if (size >= sizeof(s_temp_buffer))
    {
        fprintf(stderr, "Buffer request size %u exceeds %zu\n", size, sizeof(s_temp_buffer));
        exit(1);
    }
    return reinterpret_cast<uint8_t *>(s_temp_buffer);
}
