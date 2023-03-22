/*

 */
#include "lnArduino.h"
#include "include/lnUsbStack.h"
#include "include/lnUsbCDC.h"
#include "lnBMP_usb_descriptor.h"
#include "include/lnUsbDFUrt.h"


 extern "C"
 {
#include "version.h"
#include "gdb_packet.h"
#include "gdb_main.h"
#include "target.h"
#include "gdb_packet.h"
#include "general.h"
#include "exception.h"

}
extern "C" void pins_init();
extern void serialInit();

#define GDB_CDC_DATA_AVAILABLE (1<<0)
#define GDB_SESSION_START      (1<<1)
#define GDB_SESSION_END        (1<<2)

// Rust part
extern "C" 
{
	void rngdbstub_init();
	void rngdbstub_shutdown() ;
	void rngdbstub_run(uint32_t s, const uint8_t *d);
  void rngdbstub_poll();
}


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

#define GDB_BUFFER_SIZE 1024
uint8_t gdb_buffer[GDB_BUFFER_SIZE];
lnUsbCDC *uartCdc=NULL;

class BufferGdb
{
public:
    BufferGdb(int instance)
    {
      _instance=instance;            
      _cdc=NULL;
      _cdc=new lnUsbCDC(_instance);
      _cdc->setEventHandler(gdbCdcEventHandler,this);
      _eventGroup = new lnFastEventGroup();
      _inSession=false;
    }
    bool inSession() {return _inSession;}

    lnUsbCDC *cdc() { xAssert(_cdc);return _cdc;}

    bool takeOwnership()
    {
        
        _eventGroup->takeOwnership();
        return true;        
    }    
    static void gdbCdcEventHandler(void *cookie, int interface,lnUsbCDC::lnUsbCDCEvents event, uint32_t payload)
    {
      BufferGdb *bg=(BufferGdb *)cookie;
      xAssert(interface==bg->_instance);
      bg->cdcEventHandler(event);
    }
    uint32_t waitEvents()
    {
      return _eventGroup->waitEvents(0xffff,100);
    }
    void cdcEventHandler(lnUsbCDC::lnUsbCDCEvents event)
    {
        switch (event)
        {
          case lnUsbCDC::CDC_SET_SPEED:
              break;
          case lnUsbCDC::CDC_DATA_AVAILABLE:
              xAssert(_eventGroup);
              _eventGroup->setEvents(GDB_CDC_DATA_AVAILABLE);
              break;
          case lnUsbCDC::CDC_SESSION_START:
              _inSession=true;
              xAssert(_eventGroup);
              _eventGroup->setEvents(GDB_SESSION_START);
              break;
          case lnUsbCDC::CDC_SESSION_END:
              _inSession=false;
              xAssert(_eventGroup);
              _eventGroup->setEvents(GDB_SESSION_END);
              break;
          default:
              xAssert(0);
              break;
        }
    }
   
public:
  int      _instance;
protected:
  bool    _inSession;
  lnUsbCDC *_cdc;
  lnFastEventGroup *_eventGroup;
};


//
BufferGdb *usbGdb=NULL;
/**
 * 
 */
extern void lnSoftSystemReset(void);
/**
*/
void goDfu()
{
  Logger("Rebooting to DFU...\n");
  // pull DP to low
  lnDigitalWrite(PA12,0);
  lnPinMode(PA12,lnOUTPUT);
  lnDelayMs(50);
  // and reboot
  // Gpio marker to enter bootloader mode
  lnPinMode(PA1,lnALTERNATE_PP);
  // + set marker in ram
  uint64_t *marker=(uint64_t *)0x0000000020000000;
  *marker = 0xDEADBEEFCC00FFEEULL;
  lnSoftSystemReset();
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

  // start gdb CDC/ACM
  usbGdb =new BufferGdb(0);
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
	(void) parameters;
  Logger("Gdb task starting... \n");
  platform_init();
  pins_init();
  gdb_if_init();
  initFreeRTOS();
  Logger("Here we go... \n");
  usbGdb->takeOwnership();
  bool connected=false;
  while (true)
  {
      uint32_t ev= usbGdb->waitEvents();
      // We get here at worst every 100 ms
      if(connected)
      {
        rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
      }
      if(ev)
      {
        if(ev & GDB_SESSION_START)
        {
          rngdbstub_init();
          connected=true;
        }
        if(ev & GDB_SESSION_END)
        {
          rngdbstub_shutdown();
          connected=false;
        }
        if(ev&GDB_CDC_DATA_AVAILABLE)
        {
            while(connected)
            {
              int n= usbGdb->cdc()->read(gdb_buffer, GDB_BUFFER_SIZE);
              if(n>0)
              {
                rngdbstub_run(n,gdb_buffer);
              }else 
              {
                break;
              }
            }
        }
      }
      if(!connected)
      {
        lnDelayMs(100);
      }
  }
  xAssert(0);
}

//
//
//
 void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
  {
    Logger("%s",data); // ???
  }
//
//
//
extern "C"
{
  //
  //
  void         rngdb_send_data_c( uint32_t sz, const uint8_t *ptr)
  {
      while(sz && usbGdb->inSession())
      {
        int n=usbGdb->cdc()->write(ptr,sz);
        if(!n)
        {
          xDelay(5);      
        }
        sz-=n;
        ptr+=n;
      }
  }     
  //
  //
  void rngdb_output_flush_c()
  {
    if(usbGdb->inSession())
    {
      usbGdb->cdc()->flush();
    }
  }
}