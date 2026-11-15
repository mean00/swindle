/**
 * @file bmp_reset_pin.cpp
 * @brief SWD clock frequency accessor (ESP32-only build entry).
 *
 * `bmp_get_frequency_c()` reports the `swd_frequency` global that each tap owns, so
 * on the boards where the tap can define it next to the variable it lives there:
 *   - LN (GD32F303 / CH32V307): bmp_tap_ln.cpp
 *   - RP2040 / RP2350         : bmp_tap_rp2040.cpp
 * This file is therefore only compiled for the ESP32 taps
 * (swindle_target.cmake: BRIDGE_SRCS' ESP32 branch).
 *
 * The target nRST line used to be driven from here too. On the LN probes (GD32 /
 * CH32) it now comes from the probe board table and is driven in Rust
 * (swindle/rs/rs_swindle/src/reset_pin.rs, esprit GPIO trait), while the C++
 * implementation the ESP32 builds still need (through SwdReset / pReset) is in
 * bmp_nrst_c.cpp. That keeps exactly one definition of
 * platform_nrst_set_val_internal() per firmware.
 */
#include "esprit.h"

extern uint32_t swd_frequency;

/**
 * @brief Get the current SWD frequency.
 * @return SWD clock frequency in Hz.
 */
extern "C" uint32_t bmp_get_frequency_c()
{
    return swd_frequency;
}
// EOF


//
