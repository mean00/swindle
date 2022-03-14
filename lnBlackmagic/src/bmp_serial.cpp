/**
  Bridge between USB and Uart

*/

#include "lnArduino.h"
#include "include/lnUsbStack.h"
#include "include/lnUsbCDC.h"
#include "lnSerial.h"

#define BufferSize 512
/**

*/

#define SERIAL_EVENT 1
#define USB_EVENT    2

class BMPSerial : public xTask
{
public:
  BMPSerial(int usbInst,int serialInstance) : xTask("usbserial")
  {
    _usbInstance=usbInst;
    _serialInstance=serialInstance;
    _evGroup=new xFastEventGroup;
     _usb=new lnUsbCDC(_usbInstance);
    _serial=new lnSerial(_serialInstance);
    _connected=0;
    start();
  }

  void run()
  {
    _evGroup->takeOwnership();
    _serial->init();
    _serial->setSpeed(115200);
    _serial->setCallback(_serialCallback, this);
    _serial->enableRx(true);
    _usb->setEventHandler(gdbCdcEventHandler,this);
    while(1)
    {
      int ev=_evGroup->waitEvents(SERIAL_EVENT+USB_EVENT);
      if(ev & SERIAL_EVENT)
      {
        int n=1;
        while(n)
        {
          n=_serial->read(BufferSize,_buffer);
          if(!n) continue;
          if(n<0)
          {
            Logger("Serial error\n");
            break;
          }
          if(_connected & USB_EVENT)
          {
              _usb->write(_buffer,n);
#warning OPTIMIZE              
              _usb->flush(); // optimize
          }

        }
      }
      if(ev & USB_EVENT)
      {
          int n=1;
          while(n)
          {
            n=_usb->read(_buffer,BufferSize);
            if(n<0)
            {
              Logger("Usb error\n");
              break;
            }
            if(!n) continue;
            _serial->transmit(n,_buffer);
          }
      }
    }
  }
  static void _serialCallback(void *cookie, lnSerial::Event event)
  {
    BMPSerial *me=(BMPSerial *)cookie;
    me->serialCallback(event);
  }
  void serialCallback(lnSerial::Event event)
  {
      switch(event)
      {
          case lnSerial::dataAvailable: _evGroup->setEvents(SERIAL_EVENT);break;
          default: xAssert(0);break;

      }
  }
  static void gdbCdcEventHandler(void *cookie, int interface,lnUsbCDC::lnUsbCDCEvents event,uint32_t payload)
  {
    BMPSerial *bg=(BMPSerial *)cookie;
    xAssert(interface==bg->_usbInstance);
    bg->cdcEventHandler(event,payload);
  }
  void cdcEventHandler(lnUsbCDC::lnUsbCDCEvents event,uint32_t payload)
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
            _connected|=USB_EVENT;
            break;
        case lnUsbCDC::CDC_SESSION_END:
            _connected&=USB_EVENT;
            Logger("CDC SESSION END\n");
            break;
        default:
            xAssert(0);
            break;
      }
  }

protected:
    int         _connected;
    int         _usbInstance,_serialInstance;
    uint8_t     _buffer[BufferSize];
    lnSerial    *_serial;
    xFastEventGroup *_evGroup;
    lnUsbCDC    *_usb;

};


void serialInit()
{
  BMPSerial *serial=new BMPSerial(1,2); // CDC ACM1 to Serial port 2
}
