#include "stdint.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
extern "C"
{
#include "bmp_remote.h"
#include "src/remote.h"
}

#define UNIMPLEMENTED_STUB()                                                                                           \
    {                                                                                                                  \
        printf("UNIMPLEMENTED :%s\n", __PRETTY_FUNCTION__);                                                            \
        xAssert(0);                                                                                                    \
    }
void xAssert(int a)
{
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    exit(0);
}

extern "C" bool adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    UNIMPLEMENTED_STUB();
    return 0;
}

extern "C" uint32_t adiv5_swd_read_no_check(const uint16_t addr)
{
    UNIMPLEMENTED_STUB();
    return 0;
}

extern "C" void do_assert(const char *z)
{
    printf("FATAL ERROR :%s\n", z);
    exit(-1);
}
extern "C" void _putchar(char) {};
/**
 * @brief
 *
 */
extern "C" void bmp_set_wait_state_c(unsigned int ws)
{
    printf("Stubbed : set ws %d\n", ws);
}
/**
 * @brief
 *
 */
extern "C" unsigned int bmp_get_wait_state_c(void)
{
    printf("Stubbed : get ws %d\n", 5);
    return 5;
}

//
//  Stubs for RPC mode
//
//
#if 0
extern "C" bool bmp_rpc_init_swd_c()
{
    xAssert(0);
    return true;
}
extern "C" bool bmp_rpc_swd_in_c(uint32_t *value, uint32_t nb_bits)
{
    xAssert(0);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_in_par_c(uint32_t *value, bool *par, uint32_t nb_bits)
{
    xAssert(0);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_out_c(const uint32_t value, uint32_t nb_bits)
{
    xAssert(0);
    return true;
}
/*
 */
extern "C" bool bmp_rpc_swd_out_par_c(const uint32_t value, uint32_t nb_bits)
{
    xAssert(0);
    return true;
}

extern "C" bool bmp_adiv5_full_dp_read_c(const uint32_t device_index, const uint32_t ap_selection,
                                         const uint32_t address, int32_t *err, uint32_t *value)
{
    xAssert(0);
    return true;
}
extern "C" bool bmp_adiv5_full_dp_low_level_c()
{
    xAssert(0);
    return false;
}
#endif
extern "C"
{
    int gdb_if_init(void)
    {
        return 0;
    }
}
#if 0
extern "C" uint32_t bmp_adiv5_ap_read_c(const uint32_t device_index, const uint32_t ap_selection,
                                        const uint32_t address)
{
    xAssert(0);
    return 0;
}

extern "C" void bmp_adiv5_ap_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t address,
                                     const uint32_t value)
{
    xAssert(0);
}

extern "C" int32_t bmp_adiv5_mem_read_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                                        const uint32_t address, uint8_t *buffer, uint32_t len)
{
    xAssert(0);
    return 0;
}

extern "C" int32_t bmp_adiv5_mem_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                                         const uint32_t address, const uint32_t alin, const uint8_t *buffer,
                                         uint32_t len)

{
    xAssert(0);
    return 0;
}
#endif
extern "C"
{
    extern void remote_pin_set(uint8_t p, uint8_t s);
    extern void remote_pin_direction(uint8_t p, uint8_t is_direction);
    extern bool remote_pin_get(uint8_t p);

    void bmp_pin_set(uint8_t pin, uint8_t state)
    {
        remote_pin_set(pin, state);
    }

    bool bmp_pin_get(uint8_t pin)
    {
        return remote_pin_get(pin);
    }
    void bmp_pin_direction(uint8_t pin, uint8_t is_write)
    {
        remote_pin_direction(pin, is_write);
    }

    // This shouold be in platform.c but we put them here to minimize the change
    // in blackmagic source tree

    void remote_pin_gen(uint8_t code, uint8_t pin, uint8_t value)
    {
        uint8_t buffer[REMOTE_MAX_MSG_SIZE] = {REMOTE_SOM, REMOTE_GEN_PACKET, code, pin, value, REMOTE_EOM, 0};
        platform_buffer_write(buffer, 6);
        int length = platform_buffer_read(buffer, REMOTE_MAX_MSG_SIZE);
        if (length < 1)
        {
            DEBUG_ERROR("Invalid pin set reply\n");
            return; // false;
        }
        bool r = buffer[0] == REMOTE_RESP_OK;
        if (!r)
        {
            DEBUG_ERROR(" pin set error\n");
        }
        // return r;
    }

    void remote_pin_direction(uint8_t pin, uint8_t is_write)
    {
        remote_pin_gen('X', pin, is_write);
    }

    void remote_pin_set(uint8_t pin, uint8_t value) // pin 0 = clk, pin 1 = io
    {
        remote_pin_gen('W', pin, value);
    }
    bool remote_pin_get(uint8_t pin) // pin 0 = clk, pin 1 = io
    {
        uint8_t buffer[REMOTE_MAX_MSG_SIZE] = {REMOTE_SOM, REMOTE_GEN_PACKET, 'w', pin, REMOTE_EOM, 0};
        platform_buffer_write(buffer, 5);
        int length = platform_buffer_read(buffer, REMOTE_MAX_MSG_SIZE);
        if (length < 1)
        {
            DEBUG_ERROR("Invalid pin set reply\n");
            return false;
        }
        bool r = buffer[0] == REMOTE_RESP_OK;
        if (!r)
        {
            DEBUG_ERROR(" pin set error\n");
            xAssert(0);
        }
        else
        {
            // this is hackish
            switch (buffer[2])
            {
            case '0':
                r = 0;
                break;
            case '1':
                r = 1;
                break;
            default:
                xAssert(0);
            }
        }
        return r;
    }

    void riscv_jtag_dtm_handler(const uint8_t dev_index)
    {
    }

    float bmp_get_target_voltage_c()
    {
        return 0.0;
    }
#if 0
    const char *bmp_get_version_string()
    {
        return "lnbmpa";
    }
#endif
    size_t xPortGetFreeHeapSize(void)
    {
        return 33;
    }
    size_t xPortGetMinimumEverFreeHeapSize(void)
    {
        return 2;
        ;
    }
}
void lnSoftSystemReset()
{
}

extern "C" void stlink_adiv5_dp_init()
{
    xAssert(0);
}

extern "C" bool dap_run_cmd(const void *request_data, size_t request_length, void *response_data,
                            size_t response_length)
{
    xAssert(0);
    return false;
}

extern "C" bool bmp_rv_dm_read_c(uint8_t adr, uint32_t *value)
{
    xAssert(0);
    return false;
}
/**
 * @brief
 *
 * @param adr
 * @param value
 * @return true
 * @return false
 */
extern "C" bool bmp_rv_dm_write_c(uint8_t adr, uint32_t value)
{
    xAssert(0);
    return false;
}
extern "C" bool bmp_rv_dm_reset_c()
{
    xAssert(0);
    return false;
}
/*
 */
extern "C" void swdptap_init(void)
{
}
// -- eof --
