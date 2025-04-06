/**
  Bridge between USB and Uart

*/

#include "include/lnUsbCDC.h"
#include "include/lnUsbStack.h"
#include "lnArduino.h"
#include "lnBMP_pinout.h"
#include "lnSerial.h"

/**

*/

#define USB_EVENT_READ 2
#define USB_EVENT_WRITE 4
#define USB_EVENTS (USB_EVENT_READ + USB_EVENT_WRITE)

#define LN_BMP_BUFFER_SIZE 256
/*
 *
 *
 */
class BMPUsbLogger
{

  public:
    /**
     * @brief Construct a new BMPSerial object
     *
     * @param usbInst
     * @param serialInstance
     */
    BMPUsbLogger(int usbInst)
    {
        _usbInstance = usbInst;
        _evGroup = new lnFastEventGroup;
        _usb = new lnUsbCDC(_usbInstance);
    }
    void printOut(int n, const char *data)
    {
        _usb->write((const uint8_t *)data, n);
        _usb->flush();
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
            break;
        case lnUsbCDC::CDC_DATA_AVAILABLE:
            _evGroup->setEvents(USB_EVENT_READ);
            break;
        case lnUsbCDC::CDC_SESSION_START:
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
