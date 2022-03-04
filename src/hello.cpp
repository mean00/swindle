#include "lnArduino.h"
#include "include/lnUsbStack.h"
#include "include/lnUsbCDC.h"
#include "hello.h"

#define LED LN_SYSTEM_LED
/**
 */
void setup()
{
    lnPinMode(LED, lnOUTPUT);
}
#define MEVENT(x)                                                                                                      \
    case lnUsbStack::USB_##x: Logger(#x); break;

void helloUsbEvent(void *cookie, lnUsbStack::lnUsbStackEvents event)
{
    switch (event)
    {
        MEVENT(CONNECT)
        MEVENT(DISCONNECT)
        MEVENT(SUSPEND)
        MEVENT(RESUME)
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
          int n=cdc->read(buffer,32);
          if(n)
          {
            cdc->write((uint8_t *)">",1);
            cdc->write(buffer,n);
          }
        }
          break;
      case lnUsbCDC::CDC_SESSION_START:
          Logger("CDC SESSION START\n");
          break;
      case lnUsbCDC::CDC_SESSION_END:
          Logger("CDC SESSION END\n");
          break;
      default:
          xAssert(0);
          break;
    }
}

void loop()
{
    Logger("Starting lnTUSB Test\n");
    lnUsbStack *usb = new lnUsbStack;
    usb->init(5, descriptor);
    usb->setConfiguration(desc_hs_configuration, desc_fs_configuration, &desc_device, &desc_device_qualifier);
    usb->setEventHandler(NULL, helloUsbEvent);

    cdc=new lnUsbCDC(0);
    cdc->setEventHandler(cdcEventHandler,NULL);
    usb->start();
    while (1)
    {
        Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
    }
}
