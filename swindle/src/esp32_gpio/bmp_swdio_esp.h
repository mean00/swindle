/*
 */
#pragma once
#include "lnBMP_reset.h"
extern "C"
{
#include "driver/gpio.h"
}
// clang-format on
//
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

        gpio_set_level(_me, 1);
    }
    LN_ALWAYS_INLINE void off()
    {
        gpio_set_level(_me, 0);
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
    bool read()
    {
        return gpio_get_level(_me);
    }
    void on()
    {
        gpio_set_level(_me, 1); // on = let the pullup do its job
    }
    void off()
    {
        gpio_set_level(_me, 0); //  gnd
    }
    void set(uint32_t value)
    {
        gpio_set_level(_me, value); // on = let the pullup do its job
    }
    bool dir()
    {
        return currentDrive;
    }
    void hiZ()
    {
        input(); // let the pullup work
    }
    void output()
    {
        currentDrive = true;
    }
    void input()
    {
        gpio_set_level(_me, 1); // pullup = float
        currentDrive = false;
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

/**
 *
 */
class SwdWaitPin : public SwdOutputPin
{
  public:
    SwdWaitPin(lnBMPPins no) : SwdOutputPin(no)
    {
        _wait = true;
    }
    LN_ALWAYS_INLINE void clockOn()
    {
        if (_wait)
            swait();
        on();
    }
    LN_ALWAYS_INLINE void clockOff()
    {
        if (_wait)
            swait();
        off();
    }
    LN_ALWAYS_INLINE void wait()
    {
        if (_wait)
            swait();
    }
    LN_ALWAYS_INLINE void pulseClock()
    {
        if (_wait)
            swait();
        on();
        if (_wait)
            swait();
        off();
    }

  protected:
    bool _wait;
};

/**/
