/*

 */
#include "bmp_string.h"
#include "lnArduino.h"
#include "lnBmpTask.h"
#include "lnStopWatch.h"
extern "C"
{
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "morse.h"
#include "target.h"
#include "target_internal.h"
#include "version.h"

    extern "C" float bmp_get_target_voltage_c();
}
#undef snprintf
#define snprintf snprintf_
lnStopWatch stopWatch(1);

extern "C" int gdb_if_init(void);

void gdb_task(void *parameters);
extern void bmp_gpio_init_once();
extern "C"
{
    /**
     */
    void pins_init()
    {
        bmp_gpio_init_once();
    }

    /**
     */
    void platform_init()
    {
    }

    /**
     */
    static char buffer[10];
    const char *platform_target_voltage(void)
    {
        float v = bmp_get_target_voltage_c();
        snprintf(buffer, 9, "%2.2f v", v);
        return buffer;
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
        lnCreateTask(&gdb_task, "gdbTask", TASK_BMP_GDB_STACK_SIZE, NULL, TASK_BMP_GDB_PRIORITY);
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
        return 72 * 1000 * 1000;
    }

    /**
     */
    void platform_timeout_set(platform_timeout *t, uint32_t ms)
    {
        t->time = lnGetMs() + ms;
    }
    /**
     */
    bool platform_timeout_is_expired(const platform_timeout_s *t)
    {
#warning Take care of wrapping !
        uint32_t now = lnGetMs();
        if (now > t->time)
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
    bool platform_target_set_power(bool power)
    {
        return false;
    }
    /**
     */
    void platform_request_boot(void)
    {
    }
} // extern "C"
// EOF
