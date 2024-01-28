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
#include "riscv_debug.h"

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
    return false;
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
return false;
}
/**
 * @brief 
 * 
 * @param id 
 * @return true 
 * @return false 
 */
bool remote_rv_dm_probe(uint32_t *id)
{
    return false;
}
 // EOF