/*

 */
#include "esprit.h"
#include "bmp_net.h"
extern "C"
{
}
#include "lnLWIP.h"
// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
    void rngdbstub_poll();
}
//
//
//
// device -> host
// host -> device
/*
 *
 *
 */
// Forward declarations of extern "C" functions from the platform layer
extern "C" void bmp_io_begin_session();
extern "C" void bmp_io_end_session();
/**
 * @brief [TODO:description]
 *
 * @param sz [TODO:parameter]
 * @param ptr [TODO:parameter]
 */
extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    runnerGdb->writeData(sz, ptr);
}
/**
 * @brief [TODO:description]
 */
extern "C" void rngdb_output_flush_c()
{
    runnerGdb->flushWrite();
}
// EOF
