/**
 * @file platform_support.h
 * @brief The one header swindle C/C++ code uses from the old blackmagic tree.
 *
 * It replaces what the swindle C/C++ code used to pull out of blackmagic's
 * general.h, timing.h, target.h, adiv5.h, riscv_debug.h, jep106.h, gdb_if.h,
 * gdb_packet.h, exception.h, hex_utils.h and the hosted bmp_hosted.h/cli.h.
 * Only the pieces that are really used are kept: the implementations live in
 * Rust (`rs_swindle`) or in the swindle platform layers. The blackmagic platform
 * entry points nothing calls (SPI buses, target power, hw version, max
 * frequency, bootloader request, morse) are deliberately left out.
 *
 * This file is part of the Black Magic Debug project.
 */

#ifndef INCLUDE_PLATFORM_SUPPORT_H
#define INCLUDE_PLATFORM_SUPPORT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* --- Target handle (former target.h) ------------------------------------- */

/* The target handle is opaque here: Rust owns the definition */
typedef struct target target_s;

bool adiv5_swd_scan_targetid(uint32_t targetid);
void target_list_free(void);

/* Target misbehaves if reset using the nRST line */
#define TOPT_INHIBIT_NRST (1U << 0U)

/* --- Timing (former timing.h) -------------------------------------------- */

struct platform_timeout
{
    uint32_t time;
};

typedef struct platform_timeout platform_timeout_s;

void platform_timeout_set(platform_timeout_s *target, uint32_t ms);
bool platform_timeout_is_expired(const platform_timeout_s *target);
uint32_t platform_time_ms(void);

/* --- ADI(v5) / SWD (former adiv5.h) -------------------------------------- */

/* SWD transaction acknowledge codes */
#define SWD_ACK_OK          0x01U
#define SWD_ACK_WAIT        0x02U
#define SWD_ACK_FAULT       0x04U
#define SWD_ACK_NO_RESPONSE 0x07U

/* Request direction */
#define ADIV5_LOW_WRITE 0
#define ADIV5_LOW_READ  1

/* DP/AP register addresses */
#define ADIV5_APnDP 0x1000U

/* DP register 0x0: Abort */
#define ADIV5_DP_ABORT 0x0U

/* AP Abort Register (ABORT) */
#define ADIV5_DP_ABORT_ORUNERRCLR (1U << 4U)
#define ADIV5_DP_ABORT_WDERRCLR   (1U << 3U)
#define ADIV5_DP_ABORT_STKERRCLR  (1U << 2U)
#define ADIV5_DP_ABORT_STKCMPCLR  (1U << 1U)
/* Bit 0 is always defined as DAP Abort. */
#define ADIV5_DP_ABORT_DAPABORT (1U << 0U)

typedef struct adiv5_access_port adiv5_access_port_s;
typedef struct adiv5_debug_port adiv5_debug_port_s;

/**
 * @brief Debug port handle handed to the raw-access shim.
 *
 * Only the pieces the local SWD transport touches are kept: the sticky fault
 * raised by a failed transaction and the transaction entry points.
 */
struct adiv5_debug_port
{
    uint32_t fault;
    bool (*write_no_check)(uint16_t addr, uint32_t data);
    uint32_t (*read_no_check)(uint16_t addr);
    uint32_t (*low_access)(adiv5_debug_port_s *dp, uint8_t rnw, uint16_t addr, uint32_t value);
    void (*abort)(adiv5_debug_port_s *dp, uint32_t abort_mask);
};

void swdptap_init(void);

/* Build the 8 bit SWD request byte for a DP/AP register access */
uint8_t make_packet_request(uint8_t rnw, uint16_t addr);

/* --- RISC-V debug module (former riscv_debug.h + jep106.h) ---------------- */

#define RV_STATUS_VERSION_MASK 0x0000000fU

/* Manufacturer the single-wire debug scan looks for */
#define JEP106_MANUFACTURER_WCH 0x72aU /* Nanjing Yihuo Technology */

typedef enum riscv_debug_version
{
    RISCV_DEBUG_UNKNOWN,
    /* No Debug Module [Interface] present */
    RISCV_DEBUG_UNIMPL,
    /* Debug Module [Interface] conforming to version 0.11 of the spec */
    RISCV_DEBUG_0_11,
    /* Debug Module [Interface] conforming to version 0.13 of the spec */
    RISCV_DEBUG_0_13,
    /* Debug Module [Interface] conforming to version 1.0 of the spec */
    RISCV_DEBUG_1_0,
    /* Debug Module [Interface] not conforming to any available version of the spec */
    RISCV_DEBUG_NONSTANDARD,
} riscv_debug_version_e;

typedef struct riscv_dmi riscv_dmi_s;
struct riscv_dmi
{
    uint32_t ref_count;
    uint16_t designer_code;
    riscv_debug_version_e version;
    uint8_t dev_index;
    uint8_t idle_cycles;
    uint8_t address_width;
    uint8_t fault;
    /* True when the transport cannot tell a delivered access from a lost one */
    bool read_confirm;
    void (*prepare)(target_s *target);
    void (*quiesce)(target_s *target);
    bool (*read)(riscv_dmi_s *dmi, uint32_t address, uint32_t *value);
    bool (*write)(riscv_dmi_s *dmi, uint32_t address, uint32_t value);
    /* Invalidate any AP CSW / DP-SELECT write caches held by the transport */
    void (*invalidate_caches)(riscv_dmi_s *dmi);
};

void riscv_dmi_init(riscv_dmi_s *dmi);

/* --- Platform API -------------------------------------------------------- */

#if defined(PC_HOSTED) && PC_HOSTED
void platform_init(int argc, char **argv);
#else
void platform_init(void);
#endif
void platform_delay(uint32_t ms);
const char *platform_target_voltage(void);
void platform_nrst_set_val(bool assert);
bool platform_nrst_get_val(void);
void platform_target_clk_output_enable(bool enable);

/* --- Debug output + core helpers (former general.h) ---------------------- */

/* Platform printf for the debug macros: overwritten by miniplatform.h */
#ifndef PLATFORM_PRINTF
#define PLATFORM_PRINTF printf
#endif

#define PRINT_NOOP(...)                                                                \
    do                                                                                 \
    {                                                                                  \
    } while (false)

#ifndef ENABLE_DEBUG
#define ENABLE_DEBUG 0
#endif

#if ENABLE_DEBUG == 1
#define DEBUG_ERROR(...) PLATFORM_PRINTF(__VA_ARGS__)
#define DEBUG_WARN(...)  PLATFORM_PRINTF(__VA_ARGS__)
#else
#define DEBUG_ERROR(...) PRINT_NOOP(__VA_ARGS__)
#define DEBUG_WARN(...)  PRINT_NOOP(__VA_ARGS__)
#endif

/* --- GDB transport (former gdb_if.h) ------------------------------------- */

int gdb_if_init(void);

/* --- GDB reply channel (former gdb_packet.h) ----------------------------- */

void gdb_out(const char *str);
void gdb_outf(const char *fmt, ...);

/* --- Hex helpers (former hex_utils.h) ------------------------------------ */

static inline bool is_hex(const char x)
{
    return (x >= '0' && x <= '9') || (x >= 'A' && x <= 'F') || (x >= 'a' && x <= 'f');
}

#endif /* INCLUDE_PLATFORM_SUPPORT_H */
