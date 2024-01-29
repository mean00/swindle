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
#include "jep106.h"

#define RPC_RV_PACKET       'B'

#define RPC_RV_SCAN         's'
#define RPC_RV_DM_READ      'r'
#define RPC_RV_DM_WRITE     'w'

extern void do_assert(const char *s);
#define xAssert(x) if(!(x)) { do_assert(#x);}

uint8_t reply[REMOTE_MAX_MSG_SIZE];
bool remote_rv_dm_probe(uint32_t *id);
/**
 * @brief uint32_t encoded as ascii
 * 
 * @param p 
 * @return uint32_t 
 */
 static uint32_t c_from_ptr(const uint8_t *p)
{
    uint32_t out=0;
    if(*p>='a' && *p<='f') out=*p-'a';
    if(*p>='A' && *p<='F') out=*p-'A';
    if(*p>='0' && *p<='9') out=*p-'0';
    return out;
}
static uint32_t u8_from_ptr(const uint8_t *p)
{
    return (c_from_ptr(p+1)<<8)+c_from_ptr(p);
}
static uint32_t from_ptr(const uint8_t *p)
{
     return ((uint32_t)(u8_from_ptr(p+0))<<0) + 
            ((uint32_t)(u8_from_ptr(p+2))<<8) + 
            ((uint32_t)(u8_from_ptr(p+4))<<16) + 
            ((uint32_t)(u8_from_ptr(p+6))<<24) ;
}

#define SERIALIZE(X)  X&0xff, X>>8, X>>16, X>>24

extern bool remote_ch32_riscv_dmi_write_rs( uint32_t  address, uint32_t value );
extern bool remote_ch32_riscv_dmi_read_rs(  uint32_t  address, uint32_t *value );

#if 1
bool remote_ch32_riscv_dmi_read(riscv_dmi_s *dmi, uint32_t address, uint32_t *value)
{
    return remote_ch32_riscv_dmi_read_rs(address, value);
}
bool remote_ch32_riscv_dmi_write(riscv_dmi_s *dmi, uint32_t address, uint32_t value)
{
    return remote_ch32_riscv_dmi_write_rs(address, value);
}

#else

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
    if (length !=9)
    {
        DEBUG_ERROR("rv_dm_probe\n");
        return false;
    }
    bool r = reply[0] == REMOTE_RESP_OK;
    if (!r)
    {
        DEBUG_ERROR(" dm_probe set error\n");
        xAssert(0);
    }
    
    *id  = from_ptr(reply+2);
    return true;
}
#endif
/**
 * @brief 
 * 
 * @return true 
 * @return false 
 */
bool bmda_rvswd_scan()
{
    uint32_t id = 0;
    target_list_free();
    if (!remote_rv_dm_probe(&id))
    {
        return false;
    }
    DEBUG_ERROR("WCH : found 0x%x device\n", id);
    riscv_dmi_s *dmi = (riscv_dmi_s *)malloc(sizeof( *dmi));
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