/*

 */
 #include "lnArduino.h"
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

uint32_t swd_delay_cnt=0;

extern "C"
{
void pins_init() {
}

void platform_init()
{
}

const char *platform_target_voltage(void)
{
  return "??";
}

uint32_t platform_time_ms(void)
{
  return lnGetMs();
}


void platform_delay(uint32_t ms)
{
	lnDelayMs(ms);
}


/* This is a transplanted main() from main.c */
void main_task(void *parameters)
{
	(void) parameters;

	platform_init();

	while (true) {

			gdb_main();

	}

	/* Should never get here */
}

void user_init(void)
{
	xTaskCreate(&main_task, "main", 4*256, NULL, 2, NULL);
}


int gdb_if_init(void)
{
    return 0;
}
unsigned char gdb_if_getchar(void)
{
    return 0;
}
unsigned char gdb_if_getchar_to(int timeout)
{
  return 0;
}


void gdb_if_putchar(unsigned char c, int flush)
{

}
void platform_max_frequency_set(uint32_t freq)
{

}
uint32_t platform_max_frequency_get()
{
  return 160*1000*1000;
}


void platform_timeout_set(platform_timeout *t, uint32_t ms)
{

}
bool platform_timeout_is_expired(platform_timeout *t)
{
  return false;
}

uint32_t platform_target_voltage_sense(void)
{
  return 0;
}
int platform_hwversion(void)
{
  return 0;
}
void platform_srst_set_val(bool assert)
{

}
bool platform_srst_get_val(void)
{
  return false;
}
bool platform_target_get_power(void)
{
  return false;
}
void platform_target_set_power(bool power)
{

}
void platform_request_boot(void)
{

}
}
