/**
 * @file swindle_common_rp2040.cpp
 * @brief Common RP2040/RP2350 board initialisation shared across variants
 */

extern "C" void swindle_leds_run();

void loop()
{
    Logger("Starting picoSwindle\n");
    user_init();
    swindle_leds_run();
}
