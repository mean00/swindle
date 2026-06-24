/*

 */
#include "esprit.h"

extern "C"
{
#include "exception.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "target.h"
#include "version.h"
}
#include "bmp_net.h"
//
//
//
// device -> host
// host -> device
extern "C" void bmp_rtt_poll_c();
extern "C" void swindle_init_rtt();
extern "C" void swindle_run_rtt();
extern "C" void swindle_purge_rtt();
extern "C" bool swindle_rtt_enabled();
// device -> host
extern "C" uint32_t usbCdc_write_available();
extern "C" bool swindle_write_rtt_channel(uint32_t channel, uint32_t size, const uint8_t *data);
// host -> device
extern "C" void swindle_reinit_rtt();
extern "C" uint32_t swindle_rtt_write_available(uint32_t channel);
/*
 *
 *
 */
/**
 * @brief [TODO:description]
 *
 * @param index [TODO:parameter]
 * @param len [TODO:parameter]
 * @param data [TODO:parameter]
 */
extern "C" void swindle_rtt_send_data_to_host(unsigned int index, uint32_t len, const uint8_t *data)
{
    xAssert(index == 0);
    runnerRtt->writeData(len, data);
    runnerRtt->flushWrite();
}
/**
 * @brief [TODO:description]
 *
 * @param dex [TODO:parameter]
 * @return [TODO:return]
 */
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
    xAssert(dex == 0);
    return runnerRtt->writeBufferAvailable();
}

// EOF
