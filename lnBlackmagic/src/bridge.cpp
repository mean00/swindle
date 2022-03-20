/*

 */
 #include "lnArduino.h"
 #include "lnStopWatch.h"
 #include "lnBmpTask.h"
 #include "bmp_string.h"
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
// EOF
