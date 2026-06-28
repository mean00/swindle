/**
 * @file bridge.cpp
 * @brief GDB stub bridge — platform abstractions for BMP.
 *
 * Provides the platform glue functions required by the Black Magic
 * Debug GDB stub: timing, target-voltage sensing, pin init, etc.
 */
#include "bmp_string.h"
#include "esprit.h"
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
    /** @brief Initialise all probe GPIO pins. */
    void pins_init()
    {
        bmp_gpio_init_once();
    }

    /** @brief Board-level platform initialisation — no-op for swindle. */
    void platform_init()
    {
    }

    /**
     * @brief Read and format the target supply voltage.
     * @return A pointer to a static string like "3.30 v".
     */
    static char buffer[10];
    const char *platform_target_voltage(void)
    {
        float v = bmp_get_target_voltage_c();
        uint32_t by_10 = (uint32_t)(v * 10.f);
        uint32_t left = by_10 / 10;
        uint32_t right = by_10 - (left * 10);
        snprintf(buffer, 9, "%d.%02d v", left, right);
        return buffer;
    }

    /** @brief Get the system time in milliseconds. */
    uint32_t platform_time_ms(void)
    {
        return lnGetMs();
    }

    /** @brief Busy-wait delay. @param ms Delay in milliseconds. */
    void platform_delay(uint32_t ms)
    {
        lnDelayMs(ms);
    }

    /** @brief Stub — max frequency set (not configurable). */
    void platform_max_frequency_set(uint32_t freq)
    {
    }
    /** @brief Get the maximum supported SWJ frequency. @return 72 MHz. */
    uint32_t platform_max_frequency_get()
    {
        return 72 * 1000 * 1000;
    }

    /** @brief Set a platform timeout from now + @p ms. */
    void platform_timeout_set(platform_timeout *t, uint32_t ms)
    {
        t->time = lnGetMs() + ms;
    }
    /** @brief Check if a platform timeout has expired. */
    bool platform_timeout_is_expired(const platform_timeout_s *t)
    {
        // #warning Take care of wrapping !
        uint32_t now = lnGetMs();
        if (now > t->time)
            return true;
        return false;
    }

    /** @brief Stub — direct ADC voltage sense not supported. @return 0. */
    uint32_t platform_target_voltage_sense(void)
    {
        return 0;
    }
    /** @brief Stub — hardware version not available. @return 0. */
    int platform_hwversion(void)
    {
        return 0;
    }
    /** @brief Stub — target power control not supported. @return false. */
    bool platform_target_get_power(void)
    {
        return false;
    }
    /** @brief Stub — target power set not supported. @return false. */
    bool platform_target_set_power(bool power)
    {
        return false;
    }
    /** @brief Stub — bootloader request not supported. */
    void platform_request_boot(void)
    {
    }
} // extern "C"
// EOF
