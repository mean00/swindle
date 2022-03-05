/*

 */
 #include "lnArduino.h"

 #include "include/lnUsbStack.h"
 #include "include/lnUsbCDC.h"
 #include "lnBMP_usb_descriptor.h"

extern lnUsbCDC *cdc;

 extern "C"
 {
#include "version.h"
#include "gdb_packet.h"
#include "gdb_main.h"
#include "target.h"
#include "gdb_packet.h"
#include "morse.h"
#include "general.h"

}
static bool connected=false;
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
//
lnUsbCDC *cdc=NULL;
void cdcEventHandler(void *cookie, int interface,lnUsbCDC::lnUsbCDCEvents event)
{
  uint8_t buffer[32];

    switch (event)
    {
      case lnUsbCDC::CDC_DATA_AVAILABLE:
        {
        }
          break;
      case lnUsbCDC::CDC_SESSION_START:
          Logger("CDC SESSION START\n");
          connected=true;
          break;
      case lnUsbCDC::CDC_SESSION_END:
          connected=false;
          Logger("CDC SESSION END\n");
          break;
      default:
          xAssert(0);
          break;
    }
}



/**
*/
int gdb_if_init(void)
{
  Logger("Starting CDC \n");
  lnUsbStack *usb = new lnUsbStack;
  usb->init(5, descriptor);
  usb->setConfiguration(desc_hs_configuration, desc_fs_configuration, &desc_device, &desc_device_qualifier);
  usb->setEventHandler(NULL, helloUsbEvent);

  cdc=new lnUsbCDC(0);
  cdc->setEventHandler(cdcEventHandler,NULL);
  usb->start();
  return 0;
}
/**
*/
unsigned char gdb_if_getchar(void)
{
  uint8_t v;
  if(!connected) return 0x4;// ??
  while(1)
  {
    int n=cdc->read(&v,1);
    if(n>0)
        return v;
    lnDelayMs(1);
  }
  return 0;
}
/**
*/
unsigned char gdb_if_getchar_to(int timeout)
{
  uint8_t v;
  if(!connected) return 0x4;// ??
  while(timeout)
  {
    int n=cdc->read(&v,1);
    if(n>0)
        return v;
    lnDelayMs(1);
    timeout--;
  }
  return -1; // ??.
}


/**
*/
void gdb_if_putchar(unsigned char c, int flush)
{
    cdc->write(&c,1);
}
