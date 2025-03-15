/**
  Bridge between USB and Uart

*/

#include "include/lnUsbCDC.h"
#include "include/lnUsbStack.h"
#include "lnArduino.h"
#include "lnBMP_pinout.h"
#include "lnBmpTask.h"
#include "lnSerial.h"

/**

*/

#define SERIAL_EVENT 1
#define USB_EVENT_READ 2
#define USB_EVENT_WRITE 4
#define USB_EVENTS (USB_EVENT_READ + USB_EVENT_WRITE)

#define LN_BMP_BUFFER_SIZE 256

#if 0
#define XXD Logger
#define XXD_C Logger_chars
#else
#define XXD(...)                                                                                                       \
    {                                                                                                                  \
    }
#define XXD_C(...)                                                                                                     \
    {                                                                                                                  \
    }
#endif

/*
 *
 *
 */
class BMPUsbLogger : public lnTask
{

  public:
    /**
     * @brief Construct a new BMPSerial object
     *
     * @param usbInst
     * @param serialInstance
     */
    BMPUsbLogger(int usbInst) : lnTask("usblogger", TASK_BMP_SERIAL_PRIORITY, TASK_BMP_SERIAL_STACK_SIZE)
    {
        _usbInstance = usbInst;
        _evGroup = new lnFastEventGroup;
        _usb = new lnUsbCDC(_usbInstance);
        start();
    }

    /**
     * @brief
     *
     */
    void run()
    {
        _evGroup->takeOwnership();
        lnDelayMs(50); // let the gdb part start first
        _usb->setEventHandler(loggerCdcEventHandler, this);
        uint32_t ev = USB_EVENTS;
        bool oldbusy = true;
        while (true)
        {
            bool busy = false;
            busy = busy || pump();
            if (!busy)
            {
                if (oldbusy)
                {
                    //_usb->flush();
                }
                oldbusy = false;
                ev = _evGroup->waitEvents(USB_EVENTS);
            }
            oldbusy = true;
        }
    }
    /**
     * @brief
     *
     * @param ev
     * @return true
     * @return false
     */
    bool pump()
    {
#if KKK
        int nb = 0;
        switch ((int)_usb2serial._txing)
        {
        case 0: // idle
        {
            if ((ev & USB_EVENT_READ) == 0) // no event
                return false;
            nb = _usb->read(_usb2serial._buffer, LN_BMP_BUFFER_SIZE);
            if (nb == 0)
                return false;          // nothing to do
            _usb2serial._txing = true; // prepare for txing, will happen at next round
            _usb2serial._limit = nb;
            _usb2serial._dex = 0;
            return true;
        }
        break;
        case 1: {
            int avail = (int)(_usb2serial._limit - _usb2serial._dex);
            uint8_t *ptr = _usb2serial._buffer + _usb2serial._dex;
            XXD("<U2s:>");
            XXD_C(avail, (const char *)(ptr));
            int txed = _serial->transmitNoBlock(avail, ptr);
            if (txed <= 0)
                return false; // will wait for the tx done event

            _usb2serial._dex += txed;
            if (_usb2serial._limit == _usb2serial._dex) // all done
            {
                _usb2serial._txing = false;
                _evGroup->setEvents(USB_EVENTS); // for re-evaluation at next loop
            }
            return true;
        }
        break;
        default:
            break;
        }
#endif
        return false;
    }
    void printOut(int n, const char *data)
    {
        _usb->write((const uint8_t *)data, n);
    }
    static void loggerCdcEventHandler(void *cookie, int interface, lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
        BMPUsbLogger *bg = (BMPUsbLogger *)cookie;
        xAssert(interface == bg->_usbInstance);
        bg->cdcEventHandler(event, payload);
    }
    /**
     *
     *
     */
    void cdcEventHandler(lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
        switch (event)
        {
        case lnUsbCDC::CDC_SET_SPEED:
            Logger("CDC SET SPEED\n");
            break;
        case lnUsbCDC::CDC_DATA_AVAILABLE:
            _evGroup->setEvents(USB_EVENT_READ);
            break;
        case lnUsbCDC::CDC_SESSION_START:
            Logger("CDC SESSION START\n");
            if (!_connected)
            {
            }
            _connected = true;
            break;
        case lnUsbCDC::CDC_WRITE_AVAILABLE:
            _evGroup->setEvents(USB_EVENT_WRITE);
            break;
        case lnUsbCDC::CDC_SESSION_END: // it is unlikely we'll get that one, some prg always sets it to 1, it works one
                                        // time
            //_connected = false; The DTR acts funky!
            Logger("CDC SESSION END\n");
            break;
        default:
            xAssert(0);
            break;
        }
    }

  protected:
    bool _connected;
    int _usbInstance;
    lnUsbCDC *_usb;
    lnFastEventGroup *_evGroup;
};
/**
 *
 */
BMPUsbLogger *usbLogger = NULL;
void initCDCLogger()
{
    usbLogger = new BMPUsbLogger(LN_LOGGER_INSTANCE);
}

extern "C" void usbCdc_Logger(int n, const char *data)
{
    usbLogger->printOut(n, data);
}
// EOF
