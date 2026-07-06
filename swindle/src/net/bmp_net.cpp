/**
 * @file bmp_net.cpp
 * @brief Network GDB task for Ethernet-enabled boards (W5500 / LWIP).
 *
 * Implements gdb_task(), socket runner classes (GDB + RTT), and the
 * LLP (LwIP) callback that translates network events into runner events.
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
/** @cond DOXYGEN_SKIP */
#if 0
#define DEBUGME Logger
#else
#define DEBUGME(...)                                                                                                   \
    {                                                                                                                  \
    }
#endif
/** @endcond */
#define RUNNER_GDB_PORT 2000
#define RUNNER_RTT_PORT 2001

#include "bmp_net_gdb.h"
#include "bmp_net_rtt.h"

/**
 * @brief Initialise the GDB network interface (no-op for LWIP builds
 *        where init happens inside gdb_task).
 * @return 0 (success).
 */
extern "C" int gdb_network_init(void)
{
    return 0;
}
/** @brief Stub — FreeRTOS init (not used here; task is managed by the runner). */
void initFreeRTOS()
{
}
/** @brief Stub — GDB interface init is done inside gdb_task(). */
void gdb_if_init()
{
}
/** @brief Slot number for GDB socket runner. */
#define MAIN_GDB_SLOT 0
#define MAIN_RTT_SLOT 6
#define MAIN_SERIAL_SLOT 12

lnFastEventGroup network_eventGroup;

/**
 * @brief LWIP event callback — translates network stack events to runner events.
 * @param evt Network event (LwipDown / LwipReady).
 * @param arg User-supplied argument (unused).
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
 * @brief Main GDB task for Ethernet targets.
 *
 * Initialises GPIO, LWIP, and sets up two socket runners:
 *   - RunnerGDB (port 2000) for the GDB stub protocol
 *   - RunnerRTT (port 2001) for RTT data streaming
 *
 * @param parameters FreeRTOS task parameters (unused).
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
 * @brief Forward debug serial output to the logger.
 * @param data Data buffer.
 * @param len  Number of bytes.
 */
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data);
}
//----
#include "bmp_net_rtt.cpp"
//-----
#include "bmp_net_gdb.cpp"

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
