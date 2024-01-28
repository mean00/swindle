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

#include "gdb_packet.h"
#include "bmp_remote.h"
#include "remote/protocol_v2_defs.h"

#define RPC_RV_PACKET       'B'

#define RPC_RV_SCAN         's'
#define RPC_RV_DM_READ      'r'
#define RPC_RV_DM_WRITE     'w'

extern void do_assert(const char *s);
#define xAssert(x) if(!(x)) { do_assert(#x);}

uint8_t reply[REMOTE_MAX_MSG_SIZE];


static uint32_t from_ptr(const uint8_t *p)
{
     return ((uint32_t)(p[1])<<0) + 
            ((uint32_t)(p[2])<<8) + 
            ((uint32_t)(p[3])<<16) + 
            ((uint32_t)(p[4])<<24) ;
}

#define SERIALIZE(X)  X&0xff, X>>8, X>>16, X>>24

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
    uint8_t buffer[] = {REMOTE_SOM, RPC_RV_PACKET, RPC_RV_DM_READ,  address};
    platform_buffer_write(buffer, sizeof(buffer));
    int length = platform_buffer_read(reply, REMOTE_MAX_MSG_SIZE);
    if (length < 1)
    {
        DEBUG_ERROR("Invalid pin set reply\n");
        return false;
    }
    bool r = reply[0] == REMOTE_RESP_OK;
    if (!r)
    {
        DEBUG_ERROR(" dm_probe write error\n");
        xAssert(0);
    }    
    *value  = from_ptr(reply+1);
    return true;
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
    uint8_t buffer[] = {REMOTE_SOM, RPC_RV_PACKET, RPC_RV_DM_WRITE,  address, SERIALIZE(value)};
    platform_buffer_write(buffer, sizeof(buffer));
    int length = platform_buffer_read(reply, REMOTE_MAX_MSG_SIZE);
    if (length < 1)
    {
        DEBUG_ERROR("Invalid pin set reply\n");
        return false;
    }
    bool r = reply[0] == REMOTE_RESP_OK;
    if (!r)
    {
        DEBUG_ERROR(" dm_probe write error\n");
        xAssert(0);
    }

    return r;
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
    uint8_t buffer[] = {REMOTE_SOM, RPC_RV_PACKET, RPC_RV_SCAN,  REMOTE_EOM};
    platform_buffer_write(buffer, sizeof(buffer));
    int length = platform_buffer_read(reply, REMOTE_MAX_MSG_SIZE);
    if (length < 1)
    {
        DEBUG_ERROR("Invalid pin set reply\n");
        return false;
    }
    bool r = reply[0] == REMOTE_RESP_OK;
    if (!r)
    {
        DEBUG_ERROR(" dm_probe set error\n");
        xAssert(0);
    }
    *id  = from_ptr(reply+1);
    return true;
}
 // EOF