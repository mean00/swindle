#include "lnBMPArduino.h"
extern "C"
{

#include "ctype.h"
// #include "gdb_hostio.h"
#include "exception.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "platform.h"
#include "rtt_if.h"
#include "target.h"
#include "target_internal.h"
}
// Rust-owned logger CDC
extern "C" void rn_usb_cdc_logger(int n, const uint8_t *data);
// Rust-owned bridge CDC
extern "C" void rn_serial_bridge_write(int n, const uint8_t *data);

extern "C" void swindleRedirectLog_c(int32_t toggle)
{
    Logger("Setting redirect to usb to %d\n", toggle);
    if (toggle) {
#if defined(USE_3_CDC)
        setLogger((void (*)(int, const char *))rn_usb_cdc_logger);
#else
        setLogger((void (*)(int, const char *))rn_serial_bridge_write);
#endif
    } else {
        setLogger(NULL);
    }
    Logger("Setting redirect to usb to %d\n", toggle);
}
// EOF
