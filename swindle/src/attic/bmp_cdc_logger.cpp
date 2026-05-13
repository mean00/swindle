/**
  Bridge between USB and Uart

  NOTE: This file is kept for the `initCDCLogger()` call only.
  The actual logger CDC is now owned by Rust (cdc_logger.rs).
  `usbCdc_Logger` and `usbCdc_write_available` delegate to the Rust
  implementations via `rn_usb_cdc_logger` / `rn_usb_cdc_write_available`.
*/

#include "esprit.h"

// Rust-owned logger CDC functions
extern "C" void rn_logger_cdc_init(uint32_t instance);
extern "C" void rn_usb_cdc_logger(int n, const uint8_t *data);
extern "C" uint32_t rn_usb_cdc_write_available();

void initCDCLogger()
{
    rn_logger_cdc_init(LN_LOGGER_INSTANCE);
}

extern "C" void usbCdc_Logger(int n, const char *data)
{
    rn_usb_cdc_logger(n, (const uint8_t *)data);
}
extern "C" uint32_t usbCdc_write_available()
{
    return rn_usb_cdc_write_available();
}
// EOF
