/**
 * @file remote_rv_protocol.c
 * @author your name (you@domain.com)
 * @brief
 * @version 0.1
 * @date 2024-01-28
 *
 * @copyright Copyright (c) 2024
 *
 */
#include "general.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_internal.h"

#include "bmp_remote.h"
#include "gdb_packet.h"
#include "jep106.h"
#include "remote/protocol_v2_defs.h"

#define RPC_RV_PACKET 'B'

#define RPC_RV_SCAN 's'
#define RPC_RV_DM_READ 'r'
#define RPC_RV_DM_WRITE 'w'

#define RV_DMI_SUCCESS 0U
#define RV_DMI_FAILURE 2U

extern void do_assert(const char *s);
#define xAssert(x)                                                                                                     \
    if (!(x))                                                                                                          \
    {                                                                                                                  \
        do_assert(#x);                                                                                                 \
    }

uint8_t reply[REMOTE_MAX_MSG_SIZE];
bool remote_rv_dm_probe(uint32_t *id);

extern bool remote_ch32_riscv_dmi_write_rs(uint32_t address, uint32_t value);
extern bool remote_ch32_riscv_dmi_read_rs(uint32_t address, uint32_t *value);
extern bool remote_ch32_riscv_dmi_reset_rs();

/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
bool remote_ch32_riscv_dmi_read(riscv_dmi_s *dmi, uint32_t address, uint32_t *value)
{
    int retries = 10;
    while (1)
    {
        if (!retries)
        {
            dmi->fault = RV_DMI_FAILURE;
            return false;
        }
        bool result = remote_ch32_riscv_dmi_read_rs(address, value);
        if (result)
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
}
/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
bool remote_ch32_riscv_dmi_write(riscv_dmi_s *dmi, uint32_t address, uint32_t value)
{
    int retries = 10;
    while (1)
    {
        if (!retries)
        {
            dmi->fault = RV_DMI_FAILURE;
            return false;
        }
        bool result = remote_ch32_riscv_dmi_write_rs(address, value);
        if (result)
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
}
/**
 * @brief
 *
 * @param chip_id
 * @return true
 * @return false
 */
#define RD(a, b)                                                                                                       \
    {                                                                                                                  \
        remote_ch32_riscv_dmi_read_rs((unsigned int)a, &out);                                                          \
        printf(" READ adr =0x%x, got 0x%x expected 0x%x\n", (unsigned int)a, (unsigned int)out, (unsigned int)b);      \
    }
#define WR(a, b)                                                                                                       \
    {                                                                                                                  \
        remote_ch32_riscv_dmi_write_rs((unsigned int)a, (unsigned int)b);                                              \
    }

bool bmda_rv_dm_probe(uint32_t *chip_id)
{
    uint32_t out;
    remote_ch32_riscv_dmi_reset_rs();
    *chip_id = 0;
    //
    // init sequence, reverse eng from capture
    //----------------------------------------------
    WR(0x10, 0x80000001UL); // write DM CTROL =1

    WR(0x10, 0x80000001UL); // write DM CTRL = 0x800000001
    RD(0x11, 0x00030382UL); // read DM_STATUS
    RD(0x7f, 0x30700518UL); // read 0x7f
    *chip_id = out;         // 0x203xxxx 0x303xxxx 0x305...
    WR(0x05, 0x1ffff704UL);
    WR(0x17, 0x02200000UL);
    RD(0x04, 0x30700518UL);
    RD(0x05, 0x1ffff704UL);
    return ((*chip_id) & 0x7fff) != 0x7fff;
}

/**
 * @brief
 *
 * @return true
 * @return false
 */
bool bmda_rvswd_scan()
{

    target_list_free();

    // actual scan
    uint32_t chip_id = 0;
    if (!bmda_rv_dm_probe(&chip_id))
    {
        DEBUG_ERROR("WCH : no device found\n");
        return false;
    }

    DEBUG_ERROR("WCH : found 0x%x device\n", chip_id);
    riscv_dmi_s *dmi = (riscv_dmi_s *)malloc(sizeof(*dmi));
    memset(dmi, 0, sizeof(*dmi));
    if (!dmi)
    { /* calloc failed: heap exhaustion */
        DEBUG_ERROR("calloc: failed in %s\n", __func__);
        return false;
    }
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    dmi->address_width = 8U;
    dmi->read = remote_ch32_riscv_dmi_read;
    dmi->write = remote_ch32_riscv_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}

// EOF
