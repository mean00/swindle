/**
  Bridge between USB and Uart

*/

#include "esprit.h"
#include "include/lnUsbCDC.h"
#include "include/lnUsbStack.h"
#include "bmp_devices.h"
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

class lnPump
{
  public:
    lnPump()
    {
        _txing = false;
        _dex = _limit = 0;
    }
    bool _txing;
    uint32_t _dex, _limit;
    uint8_t _buffer[LN_BMP_BUFFER_SIZE];
};
/**
 * @brief
 *
 */
class BMPSerial : public lnTask
{

  public:
    /**
     * @brief Construct a new BMPSerial object
     *
     * @param usbInst
     * @param serialInstance
     */
    BMPSerial(int usbInst, int serialInstance)
        : lnTask("usbserial", TASK_BMP_SERIAL_PRIORITY, TASK_BMP_SERIAL_STACK_SIZE)
    {
        _usbInstance = usbInst;
        _serialInstance = serialInstance;
        _evGroup = new lnFastEventGroup;
        _usb = new lnUsbCDC(_usbInstance);
        bool dma = true;
        _serial = createLnSerialRxTx(_serialInstance, LN_BMP_BUFFER_SIZE, dma); // no dma
        _connected = false;
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
        _serial->init();
        _serial->setSpeed(115200);
        _serial->setCallback(_serialCallback, this);
        _serial->enableRx(false);
        _usb->setEventHandler(gdbCdcEventHandler, this);
        uint32_t ev = SERIAL_EVENT + USB_EVENTS;
        bool oldbusy = true;
        while (true)
        {
            bool busy = false;
            busy = busy || runUsbToSerial(ev);
            busy = busy || runSerialToUsb(ev);
            if (!busy)
            {
                if (oldbusy)
                {
                    //_usb->flush();
                }
                oldbusy = false;
                ev = _evGroup->waitEvents(SERIAL_EVENT + USB_EVENTS);
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
    bool runUsbToSerial(uint32_t ev)
    {
    rerun:
        int nb = 0;
        switch ((int)_usb2serial._txing)
        {
        case 0: // idle
        {
            if ((ev & USB_EVENT_READ) == 0) // no event
                return false;
            nb = _usb->read(_usb2serial._buffer, LN_BMP_BUFFER_SIZE);
            xAssert(nb <= LN_BMP_BUFFER_SIZE);
            if (nb == 0)
                return false;          // nothing to do
            _usb2serial._txing = true; // prepare for txing, will happen at next round
            _usb2serial._limit = nb;
            _usb2serial._dex = 0;
            goto rerun;
        }
        break;
        case 1: {
            xAssert(_usb2serial._limit >= _usb2serial._dex);
            uint32_t avail = _usb2serial._limit - _usb2serial._dex;
            uint8_t *ptr = _usb2serial._buffer + _usb2serial._dex;
            XXD("<U2s:>");
            XXD_C(avail, (const char *)(ptr));
            int txed = _serial->transmitNoBlock(avail, ptr);
            if (txed <= 0)
                return false; // will wait for the tx done event

            _usb2serial._dex += txed;
            xAssert(_usb2serial._limit >= _usb2serial._dex);
            // all done ?
            if (_usb2serial._dex >= _usb2serial._limit)
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
        return false;
    }
    /**
     * @brief
     *
     * @param ev
     * @return true
     * @return false
     */
    bool runSerialToUsb(uint32_t ev)
    {
        int nb;
        uint8_t *to;
        int total = 0;

        if (!_connected) // not connected, just drop everything
        {
            _serial->purgeRx();
            return false;
        }
        switch ((int)_serial2usb._txing)
        {
        case 0: {
            if ((ev & SERIAL_EVENT) == 0)
                return false;
            nb = _serial->getReadPointer(&to);
            if (nb <= 0)
                return false; // nothing to do
            if (nb > LN_BMP_BUFFER_SIZE)
                nb = LN_BMP_BUFFER_SIZE;
            memcpy(_serial2usb._buffer, to, nb);
            _serial->consume(nb);
            _serial2usb._txing = true;
            _serial2usb._dex = 0;
            _serial2usb._limit = nb;
            return true;
        }
        case 1: {
            if (!_connected) // only pump data if we are connected
            {
                _serial2usb._txing = false;
                return false;
            }
            int available = (int)(_serial2usb._limit - _serial2usb._dex);
            xAssert(available <= LN_BMP_BUFFER_SIZE);
            uint8_t *ptr = _serial2usb._buffer + _serial2usb._dex;
            uint32_t consumed = _usb->writeNoBlock(ptr, available);
            XXD("<S2U:Avail:%d, wr:%d\n", available, consumed);
            if (!consumed)
                return false; // wait for sent done event
            _usb->flush();
            XXD("<S2U>");
            XXD_C(consumed, (const char *)(ptr));

            _serial2usb._dex += consumed;
            if (_serial2usb._dex == _serial2usb._limit)
            {
                _serial2usb._txing = false;
                _evGroup->setEvents(SERIAL_EVENT);
            }
            return true;
        }
        break;
        default:
            xAssert(0);
            break;
        }
        return false;
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
        case lnSerialCore::txDone:
            _evGroup->setEvents(SERIAL_EVENT);
            break;
        default:
            xAssert(0);
            break;
        }
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
            _serial->setSpeed((int)payload);
            break;
        case lnUsbCDC::CDC_DATA_AVAILABLE:
            _evGroup->setEvents(USB_EVENT_READ);
            break;
        case lnUsbCDC::CDC_SESSION_START:
            Logger("CDC SESSION START\n");
            if (!_connected)
            {
                _serial->enableRx(true);
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
    //
    static void gdbCdcEventHandler(void *cookie, int interface, lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
        BMPSerial *bg = (BMPSerial *)cookie;
        xAssert(interface == bg->_usbInstance);
        bg->cdcEventHandler(event, payload);
    }

  public:
    lnUsbCDC *_usb;

  protected:
    bool _connected;
    int _usbInstance, _serialInstance;
    lnSerialRxTx *_serial;

    lnFastEventGroup *_evGroup;
    lnPump _usb2serial;
    lnPump _serial2usb;
};
extern void initCDCLogger();
static BMPSerial *serial = NULL;
void serialInit()
{
#if defined(USE_RP2040) || defined(USE_RP2350)
    lnPinMode(LN_UART_RX, lnUART);
    lnPinMode(LN_UART_TX, lnUART);
#endif
    // bridge CDC ACMxxx to Serial port yy
    serial = new BMPSerial(LN_USB_INSTANCE, LN_SERIAL_INSTANCE);
#if defined(USE_3_CDC)
    initCDCLogger();
#endif
}
#if !defined(USE_3_CDC)
extern "C" void usbCdc_Logger(int n, const char *data)
{
    // thread safe ?
    serial->_usb->write((const uint8_t *)data, n);
    serial->_usb->flush();
}
extern "C" uint32_t usbCdc_write_available()
{
    return serial->_usb->writeAvailable();
}
#endif
// EOF
