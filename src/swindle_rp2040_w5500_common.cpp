/**
 * deps ../private_include/w5500_gdb_task,../swindle/src/platform/rp2040/lnRP2040_pio
 * @file    swindle_rp2040_w5500_common.cpp
 * @brief   Common entry point for RP2040 + W5500 Ethernet debug probe.
 *          Board-specific pins are defined in the including .cpp file.
 * Communication with GDB is over TCP via the W5500 (socketRunner, no LWIP).
 * @copyright (C) 2025
 * @license  See license file
 */

#include "../private_include/w5500_gdb_task.h"
#include "lnBmpTask.h"

#include "stdint.h"
#include "lnGPIO.h"
#define PICO_NO_HARDWARE 1

#ifdef USE_RTT
#include "LN_RTT.h"
extern void rttLoggerFunction(int n, const char *data);
#endif
extern void bmp_gpio_init_once();

// w5500Pins and PIN_TO_USE must be defined in the including file
extern const lnW5500SPI w5500Pins;
extern const uint8_t mac[6];

// Socket runner task — runs the GDB/RTT event loop
static void socket_task(void *parameters)
{
    (void)parameters;
    Logger("Gdb: TCP task starting...\n");
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
        process_sockets(runnerGdb, global_events, local_events);
        process_sockets(runnerRtt, global_events, local_events);
    }
}

// setup() — called once by rp.cpp's initTask before loop()
void setup()
{
#ifdef USE_RTT
    setLogger(rttLoggerFunction);
    LN_RTT_Init();
#endif

    // Initialise the W5500 Ethernet controller
    W5500LowLevel::init(0, &w5500Pins);
    W5500LowLevel::setMac(mac);
    W5500LowLevel::start(NetCb_c, NULL);
}

// user_init() — called from loop(), overrides bridge.cpp's empty version
// Initialises SWD pins and creates the socket runner FreeRTOS task.
extern "C" void user_init()
{
    Logger("Starting picoSwindle (W5500)\n");
    bmp_gpio_init_once(); // initialise SWD PIO, reset pin, ADC
    lnCreateTask(&socket_task, "socketTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
}

#include "swindle_common_rp2040.cpp"
