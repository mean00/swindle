/**
 * @brief Software-controlled target reset line.
 *
 * Wraps a GPIO pin to assert/de-assert the target's nRST signal,
 * and supports high-impedance mode when sharing the line with other drivers.
 */
/**
 * @file lnBMP_reset.h
 * @brief Reset pin control base class
 */

#include "esprit.h"
#include "lnBMP_pins.h"
#include "lnGPIO.h"
#pragma once
class SwdReset
{
  public:
    /** @brief Construct a reset controller on the given pin. */
    SwdReset(lnBMPPins no);

    /** @brief Assert nRST (drive low). */
    void on();

    /** @brief Configure the pin as a push-pull output. */
    void setup();

    /** @brief Set the pin to high-impedance (input, no pull). */
    void hiZ();

    /** @brief De-assert nRST (drive high). */
    void off();

    /** @brief Get the current reset state. @return true if asserted, false otherwise. */
    bool state()
    {
        return _state;
    }

  protected:
    lnPin _me;   /**< Underlying GPIO pin. */
    bool _state; /**< Current reset assertion state. */
};
