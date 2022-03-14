/*

 */
#include "lnArduino.h"
#include "include/lnUsbStack.h"
#include "include/lnUsbCDC.h"
#include "lnBMP_usb_descriptor.h"


 extern "C"
 {
#include "version.h"
#include "gdb_packet.h"
#include "gdb_main.h"
#include "target.h"
#include "gdb_packet.h"
#include "general.h"

}
extern "C" void pins_init();
extern void serialInit();

#define MEVENT(x)                                                                                                      \
    case lnUsbStack::USB_##x: Logger(#x);
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
        default: xAssert(0); break;
    }
}

#define GDB_BUFFER_SIZE 64
lnUsbCDC *uartCdc=NULL;
class BufferGdb
{
public:
    BufferGdb(int instance)
    {
      _instance=instance;
      _head=_tail=0;
      _connected=false;
      _cdc=NULL;
      _cdc=new lnUsbCDC(_instance);
      _cdc->setEventHandler(gdbCdcEventHandler,this);
    }
    bool connected() {return _connected;}
    static void gdbCdcEventHandler(void *cookie, int interface,lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
      BufferGdb *bg=(BufferGdb *)cookie;
      xAssert(interface==bg->_instance);
      bg->cdcEventHandler(event);
    }
    void cdcEventHandler(lnUsbCDC::lnUsbCDCEvents event)
    {
        switch (event)
        {
          case lnUsbCDC::CDC_SET_SPEED:
          case lnUsbCDC::CDC_DATA_AVAILABLE:
              break;
          case lnUsbCDC::CDC_SESSION_START:
              Logger("CDC SESSION START\n");
              _connected=true;
              break;
          case lnUsbCDC::CDC_SESSION_END:
              _connected=false;
              Logger("CDC SESSION END\n");
              break;
          default:
              xAssert(0);
              break;
        }
    }
    /**
    */
    unsigned char getChar(int timeout)
    {
        while(1)
        {
          if(!_connected)
          {
            lnDelayMs(100);
            return 0x4;// ??
          }
          if(_tail==_head) // empty
          {
            _tail=_head=0;
            int n=_cdc->read(_buffer,GDB_BUFFER_SIZE);
            if(n<0) return -1;
            if(n==0)
            {
              lnDelayMs(1);
              switch(timeout)
              {
                case 0: return -1; break;
                case -1: break;
                default: timeout--; break;
              }
              continue;
            }
            _tail+=n;
          }
          unsigned char out=_buffer[_head];
          _head++;
          return out;
        }
    }

    void putChar(unsigned char c,bool flush)
    {
        if(!_connected) return;
        if(_cdc->write(&c,1)<0)
        {
                Logger("Error writing to USB\n");
                return;
        }
        if(flush)
          _cdc->flush();
    }
public:
  int      _instance;
protected:
  int     _head,_tail;
  uint8_t _buffer[GDB_BUFFER_SIZE];
  lnUsbCDC *_cdc;
  bool      _connected;
};


//
BufferGdb *usbGdb=NULL;

/**
*/
extern "C" int gdb_if_init(void)
{
  Logger("Starting CDC \n");
  lnUsbStack *usb = new lnUsbStack;
  usb->init(5, descriptor);
  usb->setConfiguration(desc_hs_configuration, desc_fs_configuration, &desc_device, &desc_device_qualifier);
  usb->setEventHandler(NULL, helloUsbEvent);

  // start gdb CDC/ACM
  usbGdb =new BufferGdb(0);
  serialInit();
  usb->start();
  return 0;
}
/**
*/
extern "C"  unsigned char gdb_if_getchar(void){             return usbGdb->getChar(-1);}
extern "C"  unsigned char gdb_if_getchar_to(int timeout){   return usbGdb->getChar(timeout);}
extern "C"  void          gdb_if_putchar(unsigned char c, int flush){  usbGdb->putChar(c,flush);}

/* This is a transplanted main() from main.c */
void main_task(void *parameters)
{
	(void) parameters;
  Logger("Gdb task starting... \n");
	platform_init();
  pins_init();
  gdb_if_init();
  Logger("Here we go... \n");
	while (true)
  {
			gdb_main();
      if(!usbGdb->connected())
      {
        lnDelayMs(100);
      }
	}
  xAssert(0);
}
