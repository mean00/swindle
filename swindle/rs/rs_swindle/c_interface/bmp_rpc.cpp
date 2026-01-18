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
#include "swindle_build_options.h"

#ifndef CONFIG_IDF_TARGET
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value);
#endif

adiv5_debug_port_s remote_dp = {
#ifdef CONFIG_IDF_TARGET
    .write_no_check = NULL,
    .read_no_check = NULL,
#else
    .write_no_check = ln_adiv5_swd_write_no_check,
    .read_no_check = ln_adiv5_swd_read_no_check,
#endif
    .dp_read = adiv5_swd_read,
    .error = adiv5_swd_clear_error,
#ifdef CONFIG_IDF_TARGET
    .low_access = NULL,
#else
    .low_access = ln_adiv5_swd_raw_access,
#endif
    .abort = adiv5_swd_abort,
    .ap_read = adiv5_ap_reg_read,
    .ap_write = adiv5_ap_reg_write,
    .mem_read = adiv5_mem_read_bytes,
    .mem_write = adiv5_mem_write_bytes,
};
/**
 */
extern "C" void bmp_clear_dp_fault_c()
{
    remote_dp.fault = 0;
}
/**
 */
extern "C" int32_t semihosting_request(target_s *target, uint32_t syscall, uint32_t r1)
{
    return -1;
}

/*
 */

extern "C" bool bmp_rpc_init_swd_c()
{
    swdptap_init();
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

/**
        \fn ln_adiv5_swd_read_no_check

 */
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value);
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value);
/**
 */
extern "C" uint32_t bmp_adiv5_swd_raw_access_c(const uint8_t rnw, const uint16_t addr, const uint32_t value,
                                               uint32_t *fault)
{
    TRY(EXCEPTION_ALL)
    {
#ifdef CONFIG_IDF_TARGET
        xAssert(0);
#else
        uint32_t ret = ln_adiv5_swd_raw_access(&remote_dp, rnw, addr, value);
        *fault = remote_dp.fault;
        return ret;
#endif
    }
    CATCH()
    {
    default:
        *fault = 0xff; // special marker for raise
        return 0;
    }
    return 0;
}
//--
extern "C" bool bmp_adiv5_swd_write_no_check_c(const uint16_t addr, const uint32_t data)
{
#ifdef CONFIG_IDF_TARGET
    xAssert(0);
    return false;
#else
    return ln_adiv5_swd_write_no_check(addr, data);
#endif
}
//---
extern "C" uint32_t bmp_adiv5_swd_read_no_check_c(const uint16_t addr)
{
#ifdef CONFIG_IDF_TARGET
    xAssert(0);
    return false;
#else
    return ln_adiv5_swd_read_no_check(addr);
#endif
}
//---
extern "C" void bmp_raw_swd_write_c(uint32_t tick, uint32_t value)
{
#ifdef CONFIG_IDF_TARGET
    xAssert(0);
#endif
    ln_raw_swd_write(tick, value);
}
//----

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
                                         const uint32_t address, const uint32_t align, const uint8_t *buffer,
                                         uint32_t len)
{
    AP_PREAMBLE()
    remote_ap.csw = csw;
    adiv5_mem_write_aligned(&remote_ap, address, (const void *)buffer, (size_t)len, (align_e)align);
    return remote_dp.fault;
}

extern "C" const char *bmp_get_version_string(void)
{
    return "Version " LN_BMP_VERSION " build with flags " SWINDLE_BUILD_OPTIONS " Generated on " LN_BMP_GEN_DATE
           " (hash " LN_BMP_GIT_HASH ")\n";
}

// EOF
