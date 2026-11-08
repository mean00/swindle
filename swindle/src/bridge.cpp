/**
 * @file bridge.cpp
 * @brief GDB stub bridge — platform abstractions for BMP.
 *
 * Provides the platform glue functions required by the Black Magic
 * Debug GDB stub: timing, target-voltage sensing, pin init, etc.
 */
#include "bmp_string.h"
#include "bmp_pinout.h"
#include "esprit.h"
#include "lnBmpTask.h"
#include "lnStopWatch.h"
extern "C"
{
#include "platform_support.h"
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

    uint32_t platform_get_adc_nreset_pin(void)
    {
#ifdef PIN_ADC_NRESET_DIV_BY_TWO
        return (uint32_t)PIN_ADC_NRESET_DIV_BY_TWO;
#else
        return 0;
#endif
    }

    float platform_get_adc_nreset_multiplier(void)
    {
#ifdef PIN_ADC_NRESET_MULTIPLIER
        return (float)PIN_ADC_NRESET_MULTIPLIER;
#else
        return 1.0f;
#endif
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

    /** @brief Set a platform timeout from now + @p ms. */
    void platform_timeout_set(platform_timeout *t, uint32_t ms)
    {
        t->time = lnGetMs() + ms;
    }
    /** @brief Check if a platform timeout has expired. */
    bool platform_timeout_is_expired(const platform_timeout_s *t)
    {
        uint32_t now = lnGetMs();
        // Use signed arithmetic to handle 32-bit wrap around correctly.
        if ((int32_t)(now - t->time) >= 0)
            return true;
        return false;
    }
} // extern "C"
// EOF
