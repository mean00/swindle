/*

 */
 #include "lnArduino.h"
 #include "lnStopWatch.h"
 #include "lnBmpTask.h"
 extern "C"
 {
#include "version.h"
#include "gdb_packet.h"
#include "gdb_main.h"
#include "target.h"
#include "gdb_packet.h"
#include "morse.h"
#include "general.h"
#include "target_internal.h"

}

lnStopWatch stopWatch(1);

extern "C" int gdb_if_init(void);

void gdb_task(void *parameters);
extern void bmp_gpio_init();
extern "C"
{
/**
*/
void pins_init()
{
  bmp_gpio_init();
}

/**
*/
void platform_init()
{
}

/**
*/
const char *platform_target_voltage(void)
{
  return "??";
}

/**
*/
uint32_t platform_time_ms(void)
{
  return lnGetMs();
}


/**
*/
void platform_delay(uint32_t ms)
{
	lnDelayMs(ms);
}


/**
*/
void user_init(void)
{
	xTaskCreate(&gdb_task, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY, NULL);
}

/**
*/
void platform_max_frequency_set(uint32_t freq)
{

}
/**
*/
uint32_t platform_max_frequency_get()
{
  return 72*1000*1000;
}


/**
*/
void platform_timeout_set(platform_timeout *t, uint32_t ms)
{
  t->time=lnGetMs()+ms;
}
/**
*/
bool platform_timeout_is_expired(platform_timeout *t)
{
#warning Take care of wrapping !
  uint32_t now=lnGetMs();
  if(now>t->time) return true;
  return false;
}

/**
*/
uint32_t platform_target_voltage_sense(void)
{
  return 0;
}
/**
*/
int platform_hwversion(void)
{
  return 0;
}
/**
*/
bool platform_target_get_power(void)
{
  return false;
}
/**
*/
void platform_target_set_power(bool power)
{

}
/**
*/
void platform_request_boot(void)
{

}
void abort()
{
  deadEnd(0x33);
}
} // extern "C"

int maxWrappedString=0;
class stringWrapper
{
public:
  stringWrapper()
  {
      _limit=256;
      _st=new char[_limit];
      _st[0]=0;
  }
  ~stringWrapper()
  {
    // we dont free _st here NOT A MISTAKE!
  }
  void doubleLimit()
  {
    _limit*=2;
    if(_limit>maxWrappedString) maxWrappedString=_limit;
    char *st2=new char[_limit];
    strcpy(st2,_st);
    free(_st);
    _st=st2;
  }
  void append(const char *a)
  {
    int l=strlen(a);
    if( (strlen(_st)+l+1)>=_limit)
    {  // increase
      doubleLimit();
    }
    strcat(_st,a);
  }

  void appendHex32(const uint32_t value)
  {
    int l=strlen(_st)+1;
    if( (strlen(_st)+8+1)>=_limit)
    {  // increase
      doubleLimit();
    }
    char hex32[9];
    sprintf(hex32, "%08" PRIx32, value);
    strcat(_st,hex32);
  }

  char *string() {return _st;}
  char *_st;
  int _limit;
};


static void map_ram(stringWrapper &wrapper, struct target_ram *ram)
{
  wrapper.append("<memory type=\"ram\" start=\"0x");
  wrapper.appendHex32(ram->start);
  wrapper.append("\" length=\"0x");
  wrapper.appendHex32((uint32_t)ram->length);
  wrapper.append("\"/>");
}

static void map_flash(stringWrapper &wrapper, struct target_flash *f)
{
  wrapper.append("<memory type=\"flash\" start=\"0x");
  wrapper.appendHex32( f->start);
  wrapper.append("\" length=\"0x");
  wrapper.appendHex32((uint32_t)f->length);
  wrapper.append("\">");

  wrapper.append("<property name=\"blocksize\">0x");
  wrapper.appendHex32( (uint32_t)f->blocksize );
  wrapper.append("</property></memory>");
}

 extern "C" char *ztarget_mem_map(const target *t)
{
  stringWrapper wrapper;
  wrapper.append("<memory-map>");

	/* Map each defined RAM */
	for (struct target_ram *r = t->ram; r; r = r->next)
		  map_ram(wrapper, r);
	/* Map each defined Flash */
	for (struct target_flash *f = t->flash; f; f = f->next)
		  map_flash(wrapper, f);
  wrapper.append("</memory-map>");
  char *out=wrapper.string();
  return out;
}


// EOF
