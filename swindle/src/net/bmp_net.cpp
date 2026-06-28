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
#include "eth_pins.h"
//
#include "lnSocketRunner.h"
//
extern "C" void pins_init();
extern void serialInit();
extern "C" void bmp_io_begin_session();
extern "C" void bmp_io_end_session();
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
#define RUNNER_GDB_PORT 2000
#define RUNNER_RTT_PORT 2001

#include "bmp_net_gdb.h"
#include "bmp_net_rtt.h"

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
#define MAIN_RTT_SLOT 6
#define MAIN_SERIAL_SLOT 12

lnFastEventGroup network_eventGroup;

/**
 *
 * @param evt [TODO:parameter]
 * @param arg [TODO:parameter]
 */
void NetCb_c(lnLwipEvent evt, void *arg)
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
    lnLWIP::start(NetCb_c, NULL);
    initFreeRTOS();
    //
    network_eventGroup.takeOwnership();
    runnerGdb = new socketRunnerGdb(network_eventGroup, MAIN_GDB_SLOT);
    runnerRtt = new socketRunnerRtt(network_eventGroup, MAIN_RTT_SLOT);
    uint32_t mask = (0xffffffffUL);
    mask &= ~(socketRunner::CanWrite << 0);
    const uint32_t global_mask = (socketRunner::Up | socketRunner::Down);
    while (1)
    {
        uint32_t events = network_eventGroup.waitEvents(mask, 20);
        uint32_t global_events = events & global_mask;
        uint32_t local_events = events & (~global_mask);
        if (events)
        {
            // Logger("Events ::~~ => 0x%x\n", events);
        }
        process_sockets(runnerGdb, global_events, local_events);
        process_sockets(runnerRtt, global_events, local_events);
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
//----

/*
 *
 *
 */
socketRunnerRtt::socketRunnerRtt(lnFastEventGroup &eventGroup, uint32_t shift)
    : socketRunner(RUNNER_RTT_PORT, eventGroup, shift)
{
    Logger("RunnerRtt...\n");
    _connected = false;
    swindle_init_rtt();
}

void socketRunnerRtt::hook_connected()
{
    swindle_reinit_rtt();
    _connected = true;
}
void socketRunnerRtt::hook_disconnected()
{
    _connected = false;
}
void socketRunnerRtt::hook_poll()
{
    if (_connected) // connected to a debugger
    {
        if (cur_target) // and we are connected to a target...
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

// drop all data incoming for rtt
void socketRunnerRtt::process_incoming_data()
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
            releaseData();
        }
    }
}
/**/
socketRunnerRtt *runnerRtt = NULL;

socketRunnerGdb::socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift)
    : socketRunner(RUNNER_GDB_PORT, eventGroup, shift)
{
    _connected = false;
    Logger("RunnerGdb..\n");
}

void socketRunnerGdb::hook_connected()
{
    rngdbstub_init();
    bmp_io_begin_session();
    _connected = true;
}
void socketRunnerGdb::hook_disconnected()
{
    _connected = false;
    rngdbstub_shutdown();
    bmp_io_end_session();
}
void socketRunnerGdb::hook_poll()
{
    if (_connected) // connected to a debugger
    {
        rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
    }
}

void socketRunnerGdb::process_incoming_data()
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

socketRunnerGdb *runnerGdb = NULL;

/**
 * @brief Process pending events for a single socket runner.
 *
 * @param runner  Pointer to the socket runner instance.
 * @param global  Global events (Up/Down).
 * @param locl    Local events (per-slot bitmask).
 */
void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl)
{
    uint32_t limited = (locl >> runner->shift()) & socketRunner::Mask;
    runner->process_events(limited | global);
}

// EOF
