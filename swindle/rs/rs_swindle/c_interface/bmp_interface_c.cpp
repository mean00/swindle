#include "lnBMPArduino.h"
#include <stdlib.h>
extern "C"
{

#include "ctype.h"
#include "platform_support.h"
#include "platform.h"
}
//
// C++
bool rv_dm_probe(uint32_t *chip_id); // C++
// C
extern "C"
{
    extern "C" size_t xPortGetFreeHeapSize(void);
    extern "C" size_t xPortGetMinimumEverFreeHeapSize(void);
    extern "C" void poll_rtt(target_s *const cur_target);
    extern "C" void swindle_enable_rtt(bool);
    extern "C" void bmp_gpio_reset();
    bool shutdown_bmda;
    unsigned cortexm_wait_timeout = 2000;
    bool debug_bmp = false;

    // ---------------------------------------------------------------
    // Phase 0 instrumentation: global wire-transaction counters.
    // Incremented by the swd_template.h / rvswd_template.h hot loops.
    // ---------------------------------------------------------------
    volatile uint32_t ln_swd_tx_count = 0;
    volatile uint32_t ln_rv_tx_count = 0;

#define STUB_BUFFER_SIZE 256
    static uint8_t stub_buffer[STUB_BUFFER_SIZE];
}

/**
 * @brief Snapshot the global wire-transaction counters.
 */
extern "C" void bmp_mem_counts_c(uint32_t *swd, uint32_t *rv)
{
    *swd = ln_swd_tx_count;
    *rv = ln_rv_tx_count;
}
/**
 * @brief Zero the global wire-transaction counters.
 */
extern "C" void bmp_mem_counts_reset_c()
{
    ln_swd_tx_count = 0;
    ln_rv_tx_count = 0;
}

extern "C" void bmp_log_mem_c(bool is_write, uint32_t addr, uint32_t length)
{
    static uint32_t t0, swd0, rv0;
    if (length == 0)
    {
        t0 = lnGetUs();
        swd0 = ln_swd_tx_count;
        rv0 = ln_rv_tx_count;
    }
    else
    {
        const uint32_t elapsed = lnGetUs() - t0;
        Logger("[MEMLOG] %c addr=0x%x len=%u swd_tx=%u rv_tx=%u us=%ld\n", is_write ? 'W' : 'R', addr, length,
               ln_swd_tx_count - swd0, ln_rv_tx_count - rv0, (long)elapsed);
    }
}

extern "C" void *bmp_get_temporary_buffer(uint32_t asked)
{
    if (asked > STUB_BUFFER_SIZE)
    {
        xAssert(0);
    }
    return stub_buffer;
}

static char tmpBuffer[128];
/**
 */
extern "C" void gdb_outf(const char *fmt, ...)
{
    va_list ap;

    va_start(ap, fmt);
    vsnprintf((char *)tmpBuffer, 127, fmt, ap);
    gdb_out(tmpBuffer);
    va_end(ap);
}

extern "C" bool parse_enable_or_disable(const char *value, bool *out)
{
    const size_t value_len = strlen(value);
    if (value_len && !strncmp(value, "enable", value_len))
        *out = true;
    else if (value_len && !strncmp(value, "disable", value_len))
        *out = false;
    else
    {
        gdb_out("'enable' or 'disable' argument must be provided\n");
        return false;
    }
    return true;
}

/**
 */
extern "C" uint32_t free_heap_c()
{
    return (uint32_t)xPortGetFreeHeapSize();
}
/**
 */
extern "C" uint32_t min_free_heap_c()
{
    return (uint32_t)xPortGetMinimumEverFreeHeapSize();
}

/*
 *
 */
extern "C" void bmp_raise_exception_c()
{
}
/*
 *
 *
 */
typedef void (*exec_callback_t)(void *ctx);

extern "C" int bmp_execute_with_catch_c(exec_callback_t callback, void *ctx)
{
    callback(ctx);
    return 0;
}
/*
 *
 */
static bool reset_enabled = true;
extern "C" void bmp_enable_reset_pin_c(bool enabled)
{
    reset_enabled = enabled;
}
/*
 *
 *
 */
extern "C" void bmp_tweak_options(uint32_t *opt)
{
    if (!reset_enabled)
    {
        *opt |= TOPT_INHIBIT_NRST;
    }
}
/*
 * `DEBUG_WARN()` for Rust: the probe chain (`rs_swindle_wrapper/src/arm_probe_chain.rs`)
 * reports the designers it has no driver for through it, as the C chain it replaced did.
 */
extern "C" void bmp_debug_warn_c(const char *msg)
{
    DEBUG_WARN("%s", msg);
}

#if !PC_HOSTED
#include "lnBMP_version.h"
#include "swindle_build_options.h"

#ifndef CONFIG_IDF_TARGET
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data);
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr);
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value);
#endif

static void local_adiv5_swd_abort(adiv5_debug_port_s *dp, uint32_t abort_mask)
{
#ifndef CONFIG_IDF_TARGET
    ln_adiv5_swd_raw_access(dp, ADIV5_LOW_WRITE, ADIV5_DP_ABORT, abort_mask);
#endif
}

static adiv5_debug_port_s local_dp = {
#ifndef CONFIG_IDF_TARGET
    .write_no_check = ln_adiv5_swd_write_no_check,
    .read_no_check = ln_adiv5_swd_read_no_check,
    .low_access = ln_adiv5_swd_raw_access,
    .abort = local_adiv5_swd_abort,
#endif
};

extern "C" void bmp_clear_dp_fault_c()
{
    local_dp.fault = 0;
}

extern "C" uint32_t bmp_get_dp_fault_c()
{
    return local_dp.fault;
}

extern "C" uint32_t bmp_adiv5_swd_raw_access_c(const uint8_t rnw, const uint16_t addr, const uint32_t value,
                                               uint32_t *fault)
{
#ifdef CONFIG_IDF_TARGET
    xAssert(0);
    return 0;
#else
    uint32_t ret = ln_adiv5_swd_raw_access(&local_dp, rnw, addr, value);
    if (fault)
    {
        *fault = local_dp.fault;
    }
    return ret;
#endif
}

extern "C" const char *bmp_get_version_string(void)
{
    return "Version " LN_BMP_VERSION " build with flags " SWINDLE_BUILD_OPTIONS " Generated on " LN_BMP_GEN_DATE
           " (hash " LN_BMP_GIT_HASH ")\n";
}
#endif
// EOF
