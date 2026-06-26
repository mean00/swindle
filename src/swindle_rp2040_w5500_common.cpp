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

// WS2812
#include "../swindle/src/platform/rp2040/lnRP2040_pio.h"
#include "lnGPIO.h"
#include "ln_rp_pio.h"
#include "stdint.h"
#define PICO_NO_HARDWARE 1
#include "ws.h"
#include "xsin_table.h"

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

// Platform stubs (replacing bridge.cpp which is excluded for W5500 builds)
extern "C" const char *platform_target_voltage(void)
{
    return "0.00 v";
}

extern "C" uint32_t platform_time_ms(void)
{
    return lnGetMs();
}

extern "C" void platform_delay(uint32_t ms)
{
    lnDelayMs(ms);
}

extern "C" void platform_max_frequency_set(uint32_t freq)
{
    (void)freq;
}

extern "C" uint32_t platform_max_frequency_get()
{
    return 125 * 1000 * 1000;
}

extern "C" void platform_timeout_set(platform_timeout *t, uint32_t ms)
{
    t->time = lnGetMs() + ms;
}

extern "C" bool platform_timeout_is_expired(const platform_timeout *t)
{
    uint32_t now = lnGetMs();
    if (now > t->time)
        return true;
    return false;
}

extern "C" uint32_t platform_target_voltage_sense(void)
{
    return 0;
}

extern "C" int platform_hwversion(void)
{
    return 0;
}

extern "C" bool platform_target_get_power(void)
{
    return false;
}

extern "C" bool platform_target_set_power(bool power)
{
    (void)power;
    return false;
}

extern "C" void platform_req_boot(void)
{
}

// loop() — called repeatedly by rp.cpp's initTask
// Runs the WS2812 LED breathing animation (same as USB builds).
void loop()
{
    Logger("Starting picoSwindle\n");
    user_init();
    while (0)
    {
        lnDelayMs(100);
    }

    lnPin pin = PIN_TO_USE;
    rpPIO xpio(LN_WS2812_PIO_ENGINE);
    rpPIO_SM *xsm = xpio.getSm(0);
    lnPinModePIO(pin, LN_WS2812_PIO_ENGINE);
    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin;
    xsm->setSpeed(WS_SPEED);
    xsm->setBitOrder(false, true);
    xsm->uploadCode(sizeof(ws_program_instructions) / 2, ws_program_instructions, ws_wrap_target, ws_wrap);
    xsm->configure(pinConfig);
    xsm->setPinDir(pin, true);
    xsm->execute();

    int pix = 0;
    int inc = STEP;
    while (1)
    {
        uint32_t wait = xsin[pix] << 16;
        xsm->write(1, &wait);
        pix += inc;
        if (inc == STEP)
        {
            inc = -STEP;
            if (pix > 180 - STEP)
                pix = 180 - STEP;
        }
        else
        {
            inc = STEP;
            if (pix == 0)
                inc = STEP;
        }
        lnDelayMs(6);
    }
}
