/*

 */
 #include "lnArduino.h"
 #include "lnStopWatch.h"
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

lnStopWatch stopWatch(1);

extern "C" int gdb_if_init(void);

void main_task(void *parameters);
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
	xTaskCreate(&main_task, "main", 4*256, NULL, 2, NULL);
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
  return 160*1000*1000;
}


/**
*/
void platform_timeout_set(platform_timeout *t, uint32_t ms)
{
    stopWatch.restart(ms);
}
/**
*/
bool platform_timeout_is_expired(platform_timeout *t)
{
  if(stopWatch.elapsed())
      return true;
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
} // extern "C"
// EOF
