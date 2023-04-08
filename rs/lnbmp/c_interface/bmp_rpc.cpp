#include "lnArduino.h"
extern "C"
{

#include "general.h"
#include "ctype.h"
#include "hex_utils.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "gdb_hostio.h"
#include "target.h"
#include "target_internal.h"
#include "adiv5.h"

}


 static adiv5_debug_port_s remote_dp = {
     .ap_read = firmware_ap_read,
     .ap_write = firmware_ap_write,
     .mem_read = advi5_mem_read_bytes,
     .mem_write = adiv5_mem_write_bytes,
 };
 

/*
*/

extern "C" bool bmp_rpc_init_swd_c()
{
    remote_dp.dp_read = firmware_swdp_read;
    remote_dp.low_access = firmware_swdp_low_access;
    remote_dp.abort = firmware_swdp_abort;
    swdptap_init();
    return true;
}
/*
*/
extern "C"  bool bmp_rpc_swd_in_c(uint32_t *value, uint32_t nb_bits)
{
    *value = swd_proc.seq_in(nb_bits);
    return true;
}
/*
*/
extern "C"  bool bmp_rpc_swd_in_par_c(uint32_t *value, bool *par, uint32_t nb_bits)
{
    *par = swd_proc.seq_in_parity(value, nb_bits);
    return true;
}
/*
*/
extern "C" bool bmp_rpc_swd_out_c(const uint32_t value, uint32_t nb_bits)
{
  swd_proc.seq_out(value, nb_bits);
  return true;
}
/*
*/
extern "C" bool bmp_rpc_swd_out_par_c(const uint32_t value, uint32_t nb_bits)
{
  swd_proc.seq_out_parity(value, nb_bits);
  return true;
}
// EOF