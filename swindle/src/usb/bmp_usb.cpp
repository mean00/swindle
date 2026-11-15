/**
 * @file bmp_usb.cpp
 * @brief USB GDB task for USB-enabled boards.
 *
 * Implements gdb_task() for USB-CDC targets. Uses the Rust-owned
 * CDC-ACM (GDB stub over USB), logger CDC, and USB↔UART bridge.
 */
#include "bmp_devices.h"
#include "esprit.h"
#include "include/lnUsbCDC.h"
#include "include/lnUsbDFUrt.h"
#include "lnBmpTask.h"
#include "tusb.h" // the TUD_*_DESC_LEN / TUD_*_DESCRIPTOR macros the descriptor tables use
#include "lnBMP_usb_descriptor.h"

// The USB device stack above tinyUSB is Rust when LN_RUST_USB (usb.md P2):
// `rs_tinyusb` owns the `usbd` task, the `tud_*` callbacks and the descriptor
// plumbing. `lnUsbDcdInit()` is the remaining C piece - it creates the register-level
// `lnUsbDevice` and hooks its events to `dcd_event_*` (GD32; a no-op on CH32, whose
// DCD raises them itself). RP2040/RP2350 keep the C++ stack until P7.
#ifdef LN_RUST_USB
extern "C" void lnUsbDcdInit(void);
extern "C" void rn_usb_stack_init(const void *device, const void *qualifier, const void *fs_config,
                                 const void *hs_config, const char *const *strings, unsigned string_count);
extern "C" void rn_usb_stack_set_event_handler(void *cookie, void (*cb)(void *, int));
extern "C" void rn_usb_stack_start(void);
extern "C" void rn_usb_dfu_init(void);
#else
#include "include/lnUsbStack.h"
#include <new> // placement new for the .bss lnUsbStack instance
#endif

extern "C"
{
#include "platform_support.h"
#include "version.h"
}

#if defined(USE_RP2040) || defined(USE_RP2350)
#include "pico/bootrom.h"
#endif

extern "C" void pins_init();
extern void bmp_io_begin_session();
extern void bmp_io_end_session();
extern "C" void bmp_rtt_poll_c();
extern "C" void swindle_init_rtt();
extern "C" void swindle_run_rtt();
extern "C" void swindle_purge_rtt();
extern "C" bool swindle_rtt_enabled();
// Rust-owned logger CDC
extern "C" void rn_logger_cdc_init(uint32_t instance);
extern "C" void rn_usb_cdc_logger(int n, const uint8_t *data);
extern "C" uint32_t rn_usb_cdc_write_available();
// Rust-owned USB↔UART bridge
extern "C" void rn_serial_bridge_init(uint32_t usb_instance, int32_t serial_instance, int32_t baud);
extern "C" void rn_serial_bridge_task();
extern "C" void rn_serial_bridge_write(int n, const uint8_t *data);
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
#if defined(USE_3_CDC)
    rn_usb_cdc_logger((int)len, data);
#else
    rn_serial_bridge_write((int)len, data);
#endif
}
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
#if defined(USE_3_CDC)
    return rn_usb_cdc_write_available();
#else
    return 64;
#endif
}

// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
    void rngdbstub_poll();
    // Rust-owned GDB CDC functions
    void rngdb_cdc_init(uint32_t instance);
    void rngdb_gdb_task();
}

/** The four stack events, mirrored from `rs_tinyusb::stack` (USB_CONNECT .. USB_RESUME). */
enum
{
    USB_EV_CONNECT = 0,
    USB_EV_DISCONNECT = 1,
    USB_EV_SUSPEND = 2,
    USB_EV_RESUME = 3
};

// Note: `helloUsbEvent` executes inside the high-priority `usbd` task (`tud_*_cb`).
// It must NEVER call `Logger(...)` or acquire `LOGGER_MUTEX`, because a lower-priority
// task holding `LOGGER_MUTEX` while waiting on USB CDC TX FIFO drain would deadlock
// `usbd` and prevent `tud_task()` from servicing USB.
#ifdef LN_RUST_USB
extern "C" void helloUsbEvent(void *cookie, int event)
#else
void helloUsbEvent(void *cookie, lnUsbStack::lnUsbStackEvents event)
#endif
{
    (void)cookie;
    switch (event)
    {
    case USB_EV_CONNECT:
    case USB_EV_DISCONNECT:
    case USB_EV_SUSPEND:
    case USB_EV_RESUME:
        break;
    default:
        xAssert(0);
        break;
    }
}

// usbserial task TCB + stack: static when the board can spare the 6 KiB (lnBmpTask.h)
LN_BIG_TASK_STORAGE(usbserial, TASK_BMP_SERIAL_STACK_SIZE);

//
extern void lnSoftSystemReset(void);
#if defined(USE_RP2040) || defined(USE_RP2350)
extern void Rp2040ResetToFwUpload();
#endif
#ifndef LN_RUST_USB
/** @brief Reboot into DFU mode (USB bootloader). */
void goDfu()
{
    Logger("Rebooting to DFU...\n");
    // pull DP to low
#if !defined(USE_RP2040) && !defined(USE_RP2350)
    lnDigitalWrite(PA12, 0);
    lnPinMode(PA12, lnOUTPUT);
    lnDelayMs(50);
    // and reboot
    // Gpio marker to enter bootloader mode
    lnPinMode(PA1, lnALTERNATE_PP);
    // + set marker in ram
    uint64_t *marker = (uint64_t *)0x0000000020000000;
    *marker = 0xDEADBEEFCC00FFEEULL;
    lnSoftSystemReset();
#else
    Rp2040ResetToFwUpload();
#endif
}
#endif // LN_RUST_USB

/** @brief Initialise the USB GDB interface (CDC-ACM + optional logger + bridge). @return 0 on success. */
extern "C" bool gdb_if_init(void)
{
    Logger("Starting CDC \n");
    /* The device controller comes up first (register level, below tinyUSB), then the
     * Rust stack gets the board's descriptors and its own `usbd` task. The task stack
     * and the `lnUsbDevice` object are static, both live as long as the probe. */
#ifdef LN_RUST_USB
    lnUsbDcdInit();
    rn_usb_stack_init(&desc_device, &desc_device_qualifier, desc_fs_configuration, desc_hs_configuration, descriptor,
                      (unsigned)(sizeof(descriptor) / sizeof(descriptor[0])));
    rn_usb_stack_set_event_handler(NULL, helloUsbEvent);
#else
    /* There is exactly one USB stack per probe and it has to outlive this call
     * (it owns the TinyUSB callbacks), so keep it in .bss instead of the heap.
     * Placement new, not a plain function-local static: lnUsbStack has a virtual
     * destructor, and a static instance would drag newlib's exit machinery in
     * (__cxa_atexit + on_exits, 384 B of .bss) for a destructor nobody calls. */
    alignas(lnUsbStack) static uint8_t usb_storage[sizeof(lnUsbStack)];
    lnUsbStack *usb = new (usb_storage) lnUsbStack;
    usb->init(sizeof(descriptor) / sizeof(descriptor[0]), descriptor);
    usb->setConfiguration(desc_hs_configuration, desc_fs_configuration, &desc_device, &desc_device_qualifier);
    usb->setEventHandler(NULL, helloUsbEvent);
#endif

    // start GDB CDC/ACM via Rust-owned instance
    rngdb_cdc_init(0);
#if defined(USE_3_CDC)
    // start logger CDC via Rust-owned instance
    rn_logger_cdc_init(LN_LOGGER_INSTANCE);
#endif
#if defined(USE_RP2040) || defined(USE_RP2350)
    lnPinMode(LN_UART_RX, lnUART);
    lnPinMode(LN_UART_TX, lnUART);
#endif
    // start USB↔UART bridge via Rust-owned instance
    rn_serial_bridge_init(LN_USB_INSTANCE, LN_SERIAL_INSTANCE, 115200);
    LN_CREATE_BIG_TASK(reinterpret_cast<TaskFunction_t>(rn_serial_bridge_task), usbserial, "usbserial",
                       TASK_BMP_SERIAL_STACK_SIZE, NULL, TASK_BMP_SERIAL_PRIORITY);
    // init DFU
#ifdef LN_RUST_USB
    rn_usb_dfu_init();
#else
    lnUsbDFURT::addDFURTCb(goDfu);
#endif
#ifdef LN_RUST_USB
    rn_usb_stack_start();
#else
    usb->start();
#endif
    return true;
}
/** @brief Stub — FreeRTOS init not needed (tasks managed by the RTOS). */
void initFreeRTOS()
{
}

/**
 * @brief Main GDB task for USB targets.
 *
 * Initialises USB stack, CDC classes, then delegates the event loop
 * to the Rust GDB stub (rngdb_gdb_task).
 * @param parameters FreeRTOS task parameters (unused).
 */
void gdb_task(void *parameters)
{
    (void)parameters;
    Logger("Gdb task starting... \n");
    platform_init();
    pins_init();
    gdb_if_init();
    initFreeRTOS();
    swindle_init_rtt();
    Logger("Here we go... \n");
    // Delegate the main event loop to Rust
    rngdb_gdb_task();
    xAssert(0);
}

/** @brief Forward debug serial output to the logger. */
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data);
}
