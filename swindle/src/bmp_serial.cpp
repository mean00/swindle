/**
  Bridge between USB and Uart

*/

#include "include/lnUsbCDC.h"
#include "include/lnUsbStack.h"
#include "lnArduino.h"
#include "lnBMP_pinout.h"
#include "lnBmpTask.h"
#include "lnSerial.h"
#define BMP_SERIAL_BUFFER_SIZE 256
/**

*/

#define SERIAL_EVENT 1
#define USB_EVENT 2
/**
 *
 *
 */
class BMPSerial : public xTask
{

  public:
    BMPSerial(int usbInst, int serialInstance)
        : xTask("usbserial", TASK_BMP_SERIAL_PRIORITY, TASK_BMP_SERIAL_STACK_SIZE)
    {
        _usbInstance = usbInst;
        _serialInstance = serialInstance;
        _evGroup = new xFastEventGroup;
        _usb = new lnUsbCDC(_usbInstance);
#if 1
        bool dma = true;
        _serial = createLnSerialRxTx(_serialInstance, 128, dma); // no dma
        _connected = 0;
        start();
#endif
    }
    /**
     *
     */
    void run()
    {
        _evGroup->takeOwnership();
        lnDelayMs(50); // let the gdb part start first
        _serial->init();
        _serial->setSpeed(115200);
        _serial->setCallback(_serialCallback, this);
        _serial->enableRx(true);
        _usb->setEventHandler(gdbCdcEventHandler, this);
        int ev = SERIAL_EVENT + USB_EVENT;
        while (1)
        {

            // 1- fill in serial  buffer
            if (ev & SERIAL_EVENT)
            {
                if (!(_connected & USB_EVENT)) // not connected
                {
                    _serial->purgeRx();
                }
                else // connected
                {
                    int n;
                    uint8_t *to;
                    int total = 0;
                    while ((n = _serial->getReadPointer(&to)))
                    {
                        if (n <= 0)
                            break;
                        int consumed = 0;
                        if (_connected & USB_EVENT) // only pump data if we are connected
                        {
                            consumed = _usb->write(to, n);
                            total += consumed;
                        }
                        else
                        {
                            consumed = n;
                        }
                        if (total)
                            _serial->consume(consumed);
                    }
                    if (total)
                        _usb->flush();
                }
            }

            // 3- usb -> serial
            if (ev & USB_EVENT)
            {
                int n = 1;
                while (n)
                {
                    n = _usb->read(_usbBuffer, BMP_SERIAL_BUFFER_SIZE);
                    if (n < 0)
                    {
                        Logger("Usb error\n");
                        break;
                    }
                    if (!n)
                        break;
                    _serial->transmit(n, _usbBuffer);
                }
            }
            //    _usb->flush();
            ev = _evGroup->waitEvents(SERIAL_EVENT + USB_EVENT);
        }
    }
    /**
     *
     *
     */
    static void _serialCallback(void *cookie, lnSerialCore::Event event)
    {
        BMPSerial *me = (BMPSerial *)cookie;
        me->serialCallback(event);
    }
    /**
     *
     *
     */
    void serialCallback(lnSerialCore::Event event)
    {
        switch (event)
        {
        case lnSerialCore::dataAvailable:
            _evGroup->setEvents(SERIAL_EVENT);
            break;
        default:
            xAssert(0);
            break;
        }
    }
    static void gdbCdcEventHandler(void *cookie, int interface, lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
        BMPSerial *bg = (BMPSerial *)cookie;
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
            _serial->setSpeed(payload);
            break;
        case lnUsbCDC::CDC_DATA_AVAILABLE:
            _evGroup->setEvents(USB_EVENT);
            break;
        case lnUsbCDC::CDC_SESSION_START:
            Logger("CDC SESSION START\n");
            _connected |= USB_EVENT;
            break;
        case lnUsbCDC::CDC_SESSION_END:
            _connected &= USB_EVENT;
            Logger("CDC SESSION END\n");
            break;
        default:
            xAssert(0);
            break;
        }
    }

  protected:
    int _connected;
    int _usbInstance, _serialInstance;
    uint8_t _usbBuffer[BMP_SERIAL_BUFFER_SIZE];
    lnSerialRxTx *_serial;
    xFastEventGroup *_evGroup;
    lnUsbCDC *_usb;
};

void serialInit()
{
#ifdef USE_RP2040
    lnPinMode(GPIO4, lnUART);
    lnPinMode(GPIO5, lnUART);
#endif
    // bridge CDC ACMxxx to Serial port yy
    BMPSerial *serial = new BMPSerial(LN_USB_INSTANCE, LN_SERIAL_INSTANCE);
    EXTRA_SETUP();
}