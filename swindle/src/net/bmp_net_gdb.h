/**
 * @file bmp_net_gdb.h
 * @brief Glue between the Rust GDB stub (rngdbstub) and the GDB socket runner.
 *
 * Provides callbacks that the Rust code uses to send GDB response data
 * and flush the TCP output buffer.
 */
#include "bmp_net.h"
#include "esprit.h"
extern "C"
{
}
#include "lnLWIP.h"

/* Rust-owned GDB stub (rngdb = remote network GDB). */
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
    void rngdbstub_poll();
}

extern "C" void bmp_io_begin_session();
extern "C" void bmp_io_end_session();

/**
 * @brief Callback: Rust → network runner — queue GDB response data to the socket.
 * @param sz  Number of bytes.
 * @param ptr Data buffer.
 */
extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    runnerGdb->writeData(sz, ptr);
}

/**
 * @brief Callback: Rust → network runner — flush the TCP send buffer.
 */
extern "C" void rngdb_output_flush_c()
{
    runnerGdb->flushWrite();
}
// EOF
