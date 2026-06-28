/**
 * @file bmp_swdio_fesp.h
 * @brief ESP32 fast (dedicated-GPIO) SWDIO bit-banging classes.
 *
 * Uses dedic_gpio_cpu_ll_* for near-single-cycle bit-banging
 * on ESP32-S3 and similar cores.
 */
#pragma once
#include "lnBMP_reset.h"
extern "C"
{
#include "driver/gpio.h"
#include "soc/gpio_reg.h"
#include "soc/gpio_struct.h"
#warning HARDCODE ESP32S3 CONFIG_IDF_TARGET_ESP32S3
// #include "hal/esp32s3/include/hal/dedic_gpio_cpu_ll.h"
#include "hal/dedic_gpio_cpu_ll.h"
}
// clang-format on

extern uint32_t swd_delay_cnt;

/** Write 1 to pin via SET register. */
#define FAST_SET() REG_WRITE(GPIO_OUT_W1TS_REG, (1 << _me))
/** Write 0 to pin via CLEAR register. */
#define FAST_CLEAR() REG_WRITE(GPIO_OUT_W1TC_REG, (1 << _me))
/** Read pin level from GPIO_IN_REG. */
#define FAST_READ() ((REG_READ(GPIO_IN_REG) & (1 << _me)) != 0)

/**
 * @brief Simple output-only GPIO pin (no direction control).
 */
class SwdOutputPin
{
  public:
    SwdOutputPin(lnBMPPins no)
    {
        _me = (gpio_num_t)_mapping[no];
        on();
        lnPinMode((lnPin)_me, lnOUTPUT); // 10 Mhz
    }

    LN_ALWAYS_INLINE void on()
    {

        // gpio_set_level(_me, 1);
        FAST_SET();
    }
    LN_ALWAYS_INLINE void off()
    {
        // gpio_set_level(_me, 0);
        FAST_CLEAR();
    }
    LN_ALWAYS_INLINE void set(bool x)
    {
        if (x)
            on();
        else
            off();
    }

  protected:
    gpio_num_t _me;
};
/**
 * @brief Bidirectional SWDIO pin using dedicated GPIO LL.
 *
 * Bit position within the dedicated-GPIO bundle determines
 * which pin is driven/sampled.
 */
class SwdDirectionPin
{
  public:
    SwdDirectionPin(uint32_t bit) : _bit(1 << bit)
    {
        hiZ();
    }
    virtual ~SwdDirectionPin()
    {
    }
    LN_ALWAYS_INLINE
    bool read()
    {

        return (dedic_gpio_cpu_ll_read_in() & _bit) != 0;
    }
    LN_ALWAYS_INLINE
    void on()
    {
        // let the pull up drive it
        dedic_gpio_cpu_ll_write_mask(_bit, _bit);
    }
    LN_ALWAYS_INLINE
    void off()
    {
        dedic_gpio_cpu_ll_write_mask(_bit, 0);
    }
    LN_ALWAYS_INLINE
    void set(uint32_t value)
    {
        dedic_gpio_cpu_ll_write_mask(_bit, value ? _bit : 0);
    }
    LN_ALWAYS_INLINE
    bool dir()
    {
        return currentDrive;
    }
    LN_ALWAYS_INLINE
    void hiZ()
    {
        input(); // let the pullup work
    }
    LN_ALWAYS_INLINE
    void output()
    {
        currentDrive = true;
    }
    LN_ALWAYS_INLINE
    void input()
    {
        currentDrive = false;
        dedic_gpio_cpu_ll_write_mask(_bit, _bit);
    }
    void dir_input()
    {
        input();
    }
    void dir_output()
    {
        output();
    }
    bool currentDrive;
    uint32_t _bit;
};

/**
 * @brief SWCLK output using dedicated GPIO LL with wait-state delay.
 *
 * Each toggle calls swait() to honour the SWD frequency.
 */
class SwdWaitPin
{
  public:
    SwdWaitPin(uint32_t bit) : _bit(1 << bit)
    {
        on();
    }
    LN_ALWAYS_INLINE void on()
    {
        //    gpio_set_level(_me, 1);
        dedic_gpio_cpu_ll_write_mask(_bit, _bit);
        __asm__("" ::: "memory");
    }
    LN_ALWAYS_INLINE void off()
    {
        //    gpio_set_level(_me, 0);
        dedic_gpio_cpu_ll_write_mask(_bit, 0);
        __asm__("" ::: "memory");
    }
    LN_ALWAYS_INLINE void invClockOn()
    {
        swait();
        on();
        __asm__("" ::: "memory");
    }
    LN_ALWAYS_INLINE void invClockOff()
    {
        swait();
        off();
        __asm__("" ::: "memory");
    }
    LN_ALWAYS_INLINE void wait()
    {
        swait();
    }
    LN_ALWAYS_INLINE void invPulseClock()
    {
        swait();
        on();
        swait();
        off();
    }
    void clockOn()
    {
        xAssert(0);
    }
    void clockOff()
    {
        xAssert(0);
    }

  protected:
    const uint32_t _bit;
};
/**/
