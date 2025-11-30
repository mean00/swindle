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
#include "lnLWIP.h"
//
#warning FIXME
#define DHCP_LED PA15
//
#include "lnSocketRunner.h"
//
extern "C" void pins_init();
extern void serialInit();
extern void bmp_io_begin_session();
extern void bmp_io_end_session();
extern "C" void bmp_rtt_poll_c();
extern "C" void swindle_init_rtt();
extern "C" void swindle_run_rtt();
extern "C" void swindle_purge_rtt();
extern "C" bool swindle_rtt_enabled();
extern "C" void usbCdc_Logger(int n, const char *data);
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
extern "C" void swindle_rtt_send_data_to_host(unsigned int index, uint32_t len, const uint8_t *data)
{
}
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
    return 0;
}

// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
    void rngdbstub_poll();
}
//--
/**
 *
 */
#define NET_GDB_BUFFER_SIZE 1500

#if 0
#define DEBUGME Logger
#else
#define DEBUGME(...)                                                                                                   \
    {                                                                                                                  \
    }
#endif

class socketRunnerGdb : public socketRunner
{
  protected:
    void process_incoming_data()
    {
        uint32_t lp = 0;
        while (1)
        {
            uint32_t rd = 0;
            DEBUGME("r?\n");
            if (readData(NET_GDB_BUFFER_SIZE, _buffer, rd))
            {
                if (!rd) // no more data available
                    return;
                DEBUGME("r!%d\n", rd);
                rngdbstub_run(rd, _buffer);
                rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
                DEBUGME("d%d\n", lp++);
                // DEBUGME("Write done\n");
            }
        }
    xit:
        flushWrite();
    }

  protected:
    uint8_t _buffer[NET_GDB_BUFFER_SIZE];
};
socketRunnerGdb *runnerGdb = NULL;

/**
 * @brief [TODO:description]
 *
 * @return [TODO:return]
 */
extern "C" int gdb_network_init(void)
{
    return 0;
}
/**
 * @brief [TODO:description]
 */
void initFreeRTOS()
{
}
void gdb_if_init()
{
}
/**
 * @brief [TODO:description]
 *
 * @param parameters [TODO:parameter]
 */

void gdb_task(void *parameters)
{
    (void)parameters;
    Logger("Gdb: TCP  task starting... \n");
    platform_init();
    pins_init();
    gdb_if_init();
    initFreeRTOS();
    //
    swindle_init_rtt();
    rngdbstub_init();

    runnerGdb = new socketRunnerGdb;
    runnerGdb->run();
}
/**
 * @brief [TODO:description]
 *
 * @param data [TODO:parameter]
 * @param len [TODO:parameter]
 */
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data); // ???
}
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
