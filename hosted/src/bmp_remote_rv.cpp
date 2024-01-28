#include "stdint.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
extern "C"
{
#include "bmp_remote.h"
#include "src/remote.h"
#include "riscv_debug.h"
#include "jep106.h"
}

#define Logger printf
extern "C"
{
    bool remote_ch32_riscv_dmi_read(riscv_dmi_s *dmi, uint32_t address, uint32_t *value);
    bool remote_ch32_riscv_dmi_write(riscv_dmi_s *dmi, uint32_t address, uint32_t value);
}
/**
 * @brief 
 * 
 * @return true 
 * @return false 
 */
extern "C" bool remote_rv_dm_probe(uint32_t *id);
extern "C" bool remote_rv_dm_write_c(uint8_t adr, uint32_t value);
extern "C" bool remote_rv_dm_read_c(uint8_t adr, uint32_t value);

extern "C" bool remote_rv_rvswd_scan_c()
{
   uint32_t id = 0;
    target_list_free();
    if (!remote_rv_dm_probe(&id))
    {
        return false;
    }
    Logger("WCH : found 0x%x device\n", id);
    riscv_dmi_s *dmi = new riscv_dmi_s;
    memset(dmi, 0, sizeof(*dmi));
    if (!dmi)
    { /* calloc failed: heap exhaustion */
        Logger("calloc: failed in %s\n", __func__);
        return false;
    }
    dmi->designer_code = NOT_JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    dmi->address_width = 8U;
    dmi->read = remote_ch32_riscv_dmi_read;
    dmi->write = remote_ch32_riscv_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}
