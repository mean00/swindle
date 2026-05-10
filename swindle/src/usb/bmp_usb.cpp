/*

 */
#include "esprit.h"
#include "include/lnUsbCDC.h"
#include "include/lnUsbDFUrt.h"
#include "include/lnUsbStack.h"
#include "lnBMP_usb_descriptor.h"

extern "C"
{
#include "exception.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "target.h"
#include "version.h"
}

#if defined(USE_RP2040) || defined(USE_RP2350)
#include "pico/bootrom.h"
#endif

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
    usbCdc_Logger((int)len, (const char *)data);
}
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
    return usbCdc_write_available();
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

#define MEVENT(x)                                                                                                      \
    case lnUsbStack::USB_##x:                                                                                          \
        Logger(#x);
void helloUsbEvent(void *cookie, lnUsbStack::lnUsbStackEvents event)
{
    switch (event)
    {
        MEVENT(CONNECT)
        break;
        MEVENT(DISCONNECT)

        break;
        MEVENT(SUSPEND)
        break;
        MEVENT(RESUME)
        break;
    default:
        xAssert(0);
        break;
    }
}

lnUsbCDC *uartCdc = NULL;

//
extern void lnSoftSystemReset(void);
#if defined(USE_RP2040) || defined(USE_RP2350)
extern void Rp2040ResetToFwUpload();
#endif
/**
 */
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

/**
 */
extern "C" int gdb_if_init(void)
{
    Logger("Starting CDC \n");
    lnUsbStack *usb = new lnUsbStack;
    usb->init(7, descriptor);
    usb->setConfiguration(desc_hs_configuration, desc_fs_configuration, &desc_device, &desc_device_qualifier);
    usb->setEventHandler(NULL, helloUsbEvent);

    // start GDB CDC/ACM via Rust-owned instance
    rngdb_cdc_init(0);
    serialInit();
    // init DFU
    lnUsbDFURT::addDFURTCb(goDfu);
    usb->start();
    return 0;
}
/**
 */
void initFreeRTOS()
{
}

/*
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

//
//
//
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data); // ???
}
