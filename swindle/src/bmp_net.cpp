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

#if 0
#define DEBUGME Logger
#else
#define DEBUGME(...)                                                                                                   \
    {                                                                                                                  \
    }
#endif

class socketRunnerGdb : public socketRunner
{
  public:
    socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift) : socketRunner(eventGroup, shift)
    {
        _connected = false;
    }

  protected:
    bool _connected;
    virtual void hook_connected()
    {
        swindle_reinit_rtt();
        rngdbstub_init();
        bmp_io_begin_session();
        _connected = true;
    }
    virtual void hook_disconnected()
    {
        _connected = false;
        rngdbstub_shutdown();
        bmp_io_end_session();
    }
    virtual void hook_poll()
    {
        if (_connected) // connected to a debugger
        {
            rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
            if (cur_target)   // and we are connected to a target...
            {
                if (swindle_rtt_enabled())
                {
                    swindle_run_rtt();
                }
                else
                {
                    swindle_purge_rtt();
                }
            }
        }
    }

  protected:
    void process_incoming_data()
    {
        uint32_t lp = 0;

        while (1)
        {
            uint32_t rd = 0;
            uint8_t *data;
            if (readData(rd, &data))
            {
                if (!rd)
                    return;
                rngdbstub_run(rd, data);
                releaseData();
                DEBUGME("\td%d\n", lp++);
            }
        }
    xit:
        flushWrite();
    }

  protected:
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
#define MAIN_GDB_SLOT 0

lnFastEventGroup network_eventGroup;

/**
 *
 * @param evt [TODO:parameter]
 * @param arg [TODO:parameter]
 */
static void NetCb_c(lnLwipEvent evt, void *arg)
{
    socketRunner::RunnerEvent revt;
    switch (evt)
    {
    case LwipDown:
        revt = socketRunner::Down;
        break;
    case LwipReady:
        revt = socketRunner::Up;
        break;
    default:
        xAssert(0);
        break;
    }
    network_eventGroup.setEvents(revt);
}
/**
 *
 */
void gdb_task(void *parameters)
{
    (void)parameters;
    Logger("Gdb: TCP  task starting... \n");
    platform_init();
    pins_init();
    gdb_if_init();
    lnLWIP::start(NetCb_c, NULL);
    initFreeRTOS();
    //
    swindle_init_rtt();
    rngdbstub_init();
    network_eventGroup.takeOwnership();
    runnerGdb = new socketRunnerGdb(network_eventGroup, MAIN_GDB_SLOT); // 5 first slots
    uint32_t mask = (0xffffffffUL);
    mask &= ~(socketRunner::CanWrite << 0);
    const uint32_t global_mask = (socketRunner::Up | socketRunner::Down);
    while (1)
    {
        uint32_t events = network_eventGroup.waitEvents(mask, 20);
        uint32_t global_events = events & global_mask;
        uint32_t local_events = events & (~global_mask);
        local_events >>= MAIN_GDB_SLOT;
        local_events &= socketRunner::Mask;

        if (events)
        {
            // Logger("Events ::~~ => 0x%x\n", events);
        }
        runnerGdb->process_events(local_events | global_events);
    }
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
