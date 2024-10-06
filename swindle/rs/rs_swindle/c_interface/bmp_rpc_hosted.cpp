#include "lnBMPArduino.h"
extern "C"
{

#include "adiv5.h"
#include "ctype.h"
// #include "gdb_hostio.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "target.h"
#include "target_internal.h"
}
#include "lnBMP_version.h"

extern "C" bool remote_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern "C" uint32_t remote_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" uint32_t remote_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                                const uint32_t value);

adiv5_debug_port_s remote_dp = {
    .write_no_check = remote_adiv5_swd_write_no_check,
    .read_no_check = remote_adiv5_swd_read_no_check,
    .dp_read = adiv5_swd_read,
    .error = adiv5_swd_clear_error,
    .low_access = remote_adiv5_swd_raw_access,
    .abort = adiv5_swd_abort,
    .ap_read = adiv5_ap_reg_read,
    .ap_write = adiv5_ap_reg_write,
    .mem_read = adiv5_mem_read_bytes,
    .mem_write = adiv5_mem_write_bytes,
};

extern "C" int32_t semihosting_request(target_s *target, uint32_t syscall, uint32_t r1)
{
    return -1;
}

/*
 */

extern "C" bool bmp_rpc_init_swd_c()
{
    //
    swdptap_init();
    return true;
}
//----
extern "C" const char *bmp_get_version_string(void)
{
    return "Version " LN_BMP_VERSION " Generated on " LN_BMP_GEN_DATE " (hash " LN_BMP_GIT_HASH ")\n";
}

// EOF
