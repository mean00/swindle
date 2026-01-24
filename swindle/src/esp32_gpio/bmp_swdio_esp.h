/*
 */
#pragma once
#include "lnBMP_reset.h"
extern "C"
{
#include "soc/gpio_reg.h"
#include "soc/gpio_struct.h"
#include "driver/gpio.h"
}
// clang-format on
//
//
#define FAST_SET() REG_WRITE(GPIO_OUT_W1TS_REG, (1 << _me))
#define FAST_CLEAR() REG_WRITE(GPIO_OUT_W1TC_REG, (1 << _me))
#define FAST_READ() ((REG_READ(GPIO_IN_REG) & (1 << _me)) != 0)
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
 *
 *
 */
class SwdDirectionPin
{
  public:
    SwdDirectionPin(lnBMPPins no)
    {
        _me = (gpio_num_t)_mapping[no];
        gpio_reset_pin(_me);
        // The SWDIO is pullup + od
        gpio_config_t io_conf = {
            .pin_bit_mask = (1ULL << _me),
            .mode = GPIO_MODE_INPUT_OUTPUT_OD,     // Input + Output Open-Drain
            .pull_up_en = GPIO_PULLUP_ENABLE,      // Internal pull-up to keep it High by default
            .pull_down_en = GPIO_PULLDOWN_DISABLE, // nope
            .intr_type = GPIO_INTR_DISABLE,        // nope
        };
        gpio_config(&io_conf);
        hiZ();
    }
    virtual ~SwdDirectionPin()
    {
    }
    LN_ALWAYS_INLINE
    bool read()
    {
        //    return gpio_get_level((gpio_num_t)_me);
        return FAST_READ();
    }
    LN_ALWAYS_INLINE
    void on()
    {
        FAST_SET();
        // gpio_set_level(_me, 1); // on = let the pullup do its job
    }
    LN_ALWAYS_INLINE
    void off()
    {
        FAST_CLEAR();
        // gpio_set_level(_me, 0); //  gnd
    }
    LN_ALWAYS_INLINE
    void set(uint32_t value)
    {
        if (value)
            FAST_SET();
        else
            FAST_CLEAR();
        // gpio_set_level(_me, value); // on = let the pullup do its job
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
        // gpio_set_direction((gpio_num_t)_me, GPIO_MODE_OUTPUT);
    }
    LN_ALWAYS_INLINE
    void input()
    {
        currentDrive = false;
        FAST_SET();
        // gpio_set_level(_me, 1); // on = let the pullup do its job
        //  gpio_set_direction((gpio_num_t)_me, GPIO_MODE_INPUT);
        //  gpio_set_pull_mode((gpio_num_t)_me, GPIO_FLOATING);
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
    gpio_num_t _me;
};

/*
 *
 */
class SwdWaitPin
{
  public:
    SwdWaitPin(lnBMPPins no)
    {
        _me = (gpio_num_t)_mapping[no];
        gpio_reset_pin(_me);
        gpio_set_direction(_me, GPIO_MODE_OUTPUT);
        on();
    }
    LN_ALWAYS_INLINE void on()
    {
        //    gpio_set_level(_me, 1);
        FAST_SET();
        __asm__("" ::: "memory");
    }
    LN_ALWAYS_INLINE void off()
    {
        //    gpio_set_level(_me, 0);
        FAST_CLEAR();
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
    gpio_num_t _me;
};
/**/
