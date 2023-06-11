#include "stdint.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"

void xAssert(int a)
{
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
}

extern "C" void do_assert(const char *z)
{
    printf("FATAL ERROR :%s\n", z);
    exit(-1);
}
extern "C" void _putchar(char){};
extern "C" void bmp_set_wait_state_c(unsigned int ws)
{
}
//
//  Stubs for RPC mode
//
//
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
}
extern "C"
{
    int gdb_if_init(void)
    {
        return 0;
    }
}

// -- eof --