#include "lnBMPArduino.h"
extern "C"
{

#include "adiv5.h"
#include "ctype.h"
//#include "gdb_hostio.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "target.h"
#include "target_internal.h"
}
#include "lnBMP_version.h"

static adiv5_debug_port_s remote_dp = {
    .ap_read = firmware_ap_read,
    .ap_write = firmware_ap_write,
    .mem_read = advi5_mem_read_bytes,
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
    remote_dp.dp_read = firmware_swdp_read;
    remote_dp.low_access = firmware_swdp_low_access;
    remote_dp.abort = firmware_swdp_abort;
    swdptap_init();
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_in_c(uint32_t *value, uint32_t nb_bits)
{
    *value = swd_proc.seq_in(nb_bits);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_in_par_c(uint32_t *value, bool *par, uint32_t nb_bits)
{
    *par = swd_proc.seq_in_parity(value, nb_bits);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_out_c(const uint32_t value, uint32_t nb_bits)
{
    swd_proc.seq_out(value, nb_bits);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_out_par_c(const uint32_t value, uint32_t nb_bits)
{
    swd_proc.seq_out_parity(value, nb_bits);
    return true;
}
/**
      AP/DP part
*/

#define AP_PREAMBLE()                                                                                                  \
    remote_dp.dev_index = device_index;                                                                                \
    adiv5_access_port_s remote_ap;                                                                                     \
    remote_ap.apsel = ap_selection;                                                                                    \
    remote_ap.dp = &remote_dp;

extern "C" bool bmp_adiv5_full_dp_read_c(const uint32_t device_index, const uint32_t ap_selection,
                                         const uint16_t address, int32_t *err, uint32_t *value)
{
    AP_PREAMBLE()

    *value = adiv5_dp_read(&remote_dp, address); // firmware_swdp_read 0x40010c04
    *err = remote_dp.fault;

    return true;
}

extern "C" bool bmp_adiv5_full_dp_low_level_c(const uint32_t device_index, const uint32_t ap_selection,
                                              const uint16_t address, const uint32_t value, int32_t *err,
                                              uint32_t *outvalue)
{
    remote_dp.dev_index = device_index;
    *outvalue = remote_dp.low_access(&remote_dp, ap_selection, address, value);
    *err = remote_dp.fault;

    return true;
}

extern "C" uint32_t bmp_adiv5_ap_read_c(const uint32_t device_index, const uint32_t ap_selection,
                                        const uint32_t address)
{
    AP_PREAMBLE()
    return adiv5_ap_read(&remote_ap, address);
}

extern "C" void bmp_adiv5_ap_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t address,
                                     const uint32_t value)
{
    AP_PREAMBLE()
    return adiv5_ap_write(&remote_ap, address, value);
}

extern "C" int32_t bmp_adiv5_mem_read_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                                        const uint32_t address, uint8_t *buffer, uint32_t len)
{
    AP_PREAMBLE()
    remote_ap.csw = csw;
    adiv5_mem_read(&remote_ap, buffer, address, len);
    return remote_dp.fault;
}

extern "C" int32_t bmp_adiv5_mem_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                                         const uint32_t address, const uint32_t alin, const uint8_t *buffer,
                                         uint32_t len)
{
    AP_PREAMBLE()
    remote_ap.csw = csw;
    adiv5_mem_write_sized(&remote_ap, address, (const void *)buffer, (size_t)len, (align_e)alin);
    return remote_dp.fault;
}

extern "C" const char *bmp_get_version_string(void)
{
    return "Version " LN_BMP_VERSION " Generated on " LN_BMP_GEN_DATE " (hash " LN_BMP_GIT_HASH ")\n";
}


// EOF
