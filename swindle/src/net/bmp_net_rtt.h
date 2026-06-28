/**
 * @file bmp_net_rtt.h
 * @brief Glue between the RTT logic and the RTT socket runner.
 *
 * For network targets RTT data is forwarded over TCP port 2001 instead of USB.
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

/* Swindle RTT control functions (implemented in Rust). */
extern "C" void bmp_rtt_poll_c();
extern "C" void swindle_init_rtt();
extern "C" void swindle_run_rtt();
extern "C" void swindle_purge_rtt();
extern "C" bool swindle_rtt_enabled();
/* Device → host RTT channel (channel 0 only). */
extern "C" uint32_t usbCdc_write_available();
extern "C" bool swindle_write_rtt_channel(uint32_t channel, uint32_t size, const uint8_t *data);
/* Host → device RTT channel. */
extern "C" void swindle_reinit_rtt();
extern "C" uint32_t swindle_rtt_write_available(uint32_t channel);

/**
 * @brief Send RTT data from the target over the TCP socket to the debugger.
 * @param index Channel index (must be 0).
 * @param len   Data length.
 * @param data  Data buffer.
 */
extern "C" void swindle_rtt_send_data_to_host(unsigned int index, uint32_t len, const uint8_t *data)
{
    xAssert(index == 0);
    runnerRtt->writeData(len, data);
    runnerRtt->flushWrite();
}

/**
 * @brief Query room available in the TCP socket buffer for RTT data.
 * @param dex Channel index (must be 0).
 * @return Number of bytes the socket buffer can accept.
 */
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
    xAssert(dex == 0);
    return runnerRtt->writeBufferAvailable();
}

// EOF
