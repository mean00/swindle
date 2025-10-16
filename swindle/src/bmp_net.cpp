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

#if 0
#define GDB_BUFFER_SIZE 1024
uint8_t gdb_buffer[GDB_BUFFER_SIZE];
lnUsbCDC *uartCdc = NULL;

class BufferGdb
{
  public:
    BufferGdb(int instance)
    {
        _instance = instance;
        _cdc = NULL;
        _cdc = new lnUsbCDC(_instance);
        _cdc->setEventHandler(gdbCdcEventHandler, this);
        _eventGroup = new lnFastEventGroup();
        _inSession = false;
    }
    bool inSession()
    {
        return _inSession;
    }

    lnUsbCDC *cdc()
    {
        xAssert(_cdc);
        return _cdc;
    }

    bool takeOwnership()
    {

        _eventGroup->takeOwnership();
        return true;
    }
    static void gdbCdcEventHandler(void *cookie, int interface, lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
        BufferGdb *bg = (BufferGdb *)cookie;
        xAssert(interface == bg->_instance);
        bg->cdcEventHandler(event);
    }
    uint32_t waitEvents()
    {
        return _eventGroup->waitEvents(0xffff, GDB_MAX_POLLING_PERIOD);
    }
    void cdcEventHandler(lnUsbCDC::lnUsbCDCEvents event)
    {
        switch (event)
        {
        case lnUsbCDC::CDC_SET_SPEED:
            break;
        case lnUsbCDC::CDC_WRITE_AVAILABLE: // ignore
            break;
        case lnUsbCDC::CDC_DATA_AVAILABLE:
            xAssert(_eventGroup);
            _eventGroup->setEvents(GDB_CDC_DATA_AVAILABLE);
            break;
        case lnUsbCDC::CDC_SESSION_START:
            bmp_io_begin_session();
            _inSession = true;
            xAssert(_eventGroup);
            _eventGroup->setEvents(GDB_SESSION_START);
            break;
        case lnUsbCDC::CDC_SESSION_END:
            bmp_io_end_session();
            _inSession = false;
            xAssert(_eventGroup);
            _eventGroup->setEvents(GDB_SESSION_END);
            break;
        default:
            xAssert(0);
            break;
        }
    }

  public:
    int _instance;

  protected:
    bool _inSession;
    lnUsbCDC *_cdc;
    lnFastEventGroup *_eventGroup;
};

//
BufferGdb *usbGdb = NULL;
#endif
/**
 */
extern "C" int gdb_if_init(void)
{
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
    Logger("Gdb: TCP  task starting... \n");
    platform_init();
    pins_init();
    gdb_if_init();
    initFreeRTOS();
    //
    swindle_init_rtt();
    rngdbstub_init();
    rngdbstub_run(0, NULL);
    while (1)
    {
        rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
    }
}

//
//
//
void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
{
    Logger("%s", data); // ???
}
//
//
//
extern "C"
{
    //
    //
    void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
    {
    }
    //
    //
    void rngdb_output_flush_c()
    {
    }
}
