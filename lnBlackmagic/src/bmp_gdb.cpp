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
      _eventGroup=NULL;
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
              break;
          case lnUsbCDC::CDC_DATA_AVAILABLE:
              xAssert(_eventGroup);
              _eventGroup->setEvents(GDB_CDC_DATA_AVAILABLE);
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
      if(!_eventGroup)
      {
        _eventGroup = new lnFastEventGroup();
        _eventGroup->takeOwnership();
      }
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
              switch(timeout)
              {
                case 0: return -1; break;
                case -1: break;
                default: 
                      uint32_t ev = _eventGroup->waitEvents(GDB_CDC_DATA_AVAILABLE,50);
                      timeout--; break;
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
  lnFastEventGroup *_eventGroup;
};


//
BufferGdb *usbGdb=NULL;
/**
 * 
 */
extern void lnSoftSystemReset(void);
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
  lnSoftSystemReset();
}

/**
*/
extern "C" int gdb_if_init(void)
{
  Logger("Starting CDC \n");
  lnUsbStack *usb = new lnUsbStack;
  usb->init(6, descriptor);
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
extern "C"  unsigned char gdb_if_getchar(void){             return usbGdb->getChar(-1);}
extern "C"  unsigned char gdb_if_getchar_to(int timeout){   return usbGdb->getChar(timeout);}
extern "C"  void          gdb_if_putchar(unsigned char c, int flush){  usbGdb->putChar(c,flush);}
extern void initFreeRTOS();
//
#define BUF_SIZE 1024U
static char pbuf[BUF_SIZE + 1U];

static void bmp_poll_loop(void)
{
	SET_IDLE_STATE(false);
	while (gdb_target_running && cur_target) {
		gdb_poll_target();

		// Check again, as `gdb_poll_target()` may
		// alter these variables.
		if (!gdb_target_running || !cur_target)
			break;
		char c = gdb_if_getchar_to(0);
		if (c == '\x03' || c == '\x04')
			target_halt_request(cur_target);
		platform_pace_poll();
#ifdef ENABLE_RTT
		if (rtt_enabled)
			poll_rtt(cur_target);
#endif
	}

	SET_IDLE_STATE(true);
	size_t size = gdb_getpacket(pbuf, BUF_SIZE);
	// If port closed and target detached, stay idle
	if (pbuf[0] != '\x04' || cur_target)
		SET_IDLE_STATE(false);
	gdb_main(pbuf, sizeof(pbuf), size);
}

/* This is a transplanted main() from main.c */
void gdb_task(void *parameters)
{
	(void) parameters;
  Logger("Gdb task starting... \n");
  platform_init();
  pins_init();
  gdb_if_init();
  initFreeRTOS();
  Logger("Here we go... \n");
  while (true)
  {
      volatile exception_s e;
          TRY_CATCH (e, EXCEPTION_ALL) 
          {
          	bmp_poll_loop();
          }
          if (e.type) 
          {
          	gdb_putpacketz("EFF");
          	target_list_free();
          	gdb_outf("Uncaught exception: %s\n", e.msg);
                xAssert(0);
          }
      ;
      if(!usbGdb->connected())
      {
        lnDelayMs(100);
      }
  }
  xAssert(0);
}


 void debug_serial_send_stdout(const uint8_t *const data, const size_t len)
  {
    Logger("%s",data); // ???
  }


