
/**
 * This handles the low level RPC as used when BMP is running in hosted mode
 * It is a parralel path to the normal gdb command and is BMP specific
 */
use alloc::vec;
use alloc::vec::Vec;
use crate::util::{glog,glog1};
use crate::bmp;
use crate::encoder::{*};
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::commands::{CallbackType,exec_one,CommandTree};
use crate::glue::gdb_out_rs;
use numtoa::NumToA;
use crate::commands::rpc_commands;

/**
 * 
 */
fn rpc_message_out( message : &[u8])
{    
    encoder::raw_send_u8(&message);
    encoder::flush();    
}
fn rpc_message_out_no_flush( message : &[u8])
{    
    encoder::raw_send_u8(&message);
}

/**
 * 
 */
fn rpc_reply(code : u8, subcode: u8)
{
    let mut reply : [u8;5] =[0,0,0,0,0];
    
    let ascii=crate::parsing_util::u8_to_ascii(subcode);

    reply[0]=rpc_commands::RPC_REMOTE_RESP;
    reply[1]=code;    
    reply[2]=ascii[0];
    reply[3]=ascii[1];
    reply[4]=crate::packet_symbols::RPC_END;
    rpc_message_out(&reply);    
}
/**
 * 
 */
fn rpc_reply32(code : u8, subcode: u32)
{
    let mut reply : [u8;11] =[0,0,0,0,0,0,0,0,0,0,0];

    reply[0]=rpc_commands::RPC_REMOTE_RESP;
    reply[1]=code;    
    for i in 0..4
    {
        let ascii=crate::parsing_util::u8_to_ascii(  ((subcode>>(8*(3-i))) & 0xff) as u8);
        reply[2+i*2]=ascii[0];
        reply[3+i*2]=ascii[1];
    }
    reply[10]=crate::packet_symbols::RPC_END;
    rpc_message_out(&reply);    
}
/**
 * 
 */
#[no_mangle]
fn rpc_reply_string(code : u8, s: &[u8])
{    
    rpc_message_out_no_flush(&[rpc_commands::RPC_REMOTE_RESP,code]);
    rpc_message_out_no_flush(s);
    rpc_message_out(&[crate::packet_symbols::RPC_END]);
}
/**
 * 
 */
fn rpc_reply_hex_string(code : u8, s: &[u8])
{    
    rpc_message_out_no_flush(&[rpc_commands::RPC_REMOTE_RESP,code]);
    encoder::hexify_and_raw_send(s);
    rpc_message_out(&[crate::packet_symbols::RPC_END]);
}

/*
 */
fn rpc_hl_packet(input : &[u8]) -> bool
{
    glog("hl packet\n");
    false
}
/*
 */
#[no_mangle]
fn rpc_gen_packet(input : &[u8]) -> bool
{
    glog("\tgen packet\n");
    let mut res : u8 = 0;
    match input[0]
    {
        rpc_commands::RPC_START => {
                                        glog("rpc start session\n");
                                        rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"LNBMP");
                                        return true; 
                                    },
        rpc_commands::RPC_VOLTAGE => {
                                        glog("rpc voltage\n");
                                        rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"????"); 
                                        return true; 
                                    },
        rpc_commands::RPC_NRST_SET => {    
                                        glog("rpc rst set\n");
                                        if input.len() < 2
                                        {
                                            glog("\tmalformed NRST SET\n");
                                            return false;
                                        }   
                                        let mut enable : bool = false;
                                        if input[1]!=b'0'
                                        {
                                            enable=true;
                                        }
                                        bmp::bmp_platform_nrst_set_val(enable);  
                                        rpc_reply(rpc_commands::RPC_RESP_OK, 0);    
                                        return true;
                                    },
        rpc_commands::RPC_NRST_GET => {       
                                        glog("rpc rst get\n");
                                        let enabled : u8 = bmp::bmp_platform_nrst_get_val() as u8;
                                        rpc_reply(rpc_commands::RPC_RESP_OK, enabled);  
                                        return true;
                                    },
        rpc_commands::RPC_TARGET_CLK_OE => {       
                                        if input.len() < 2
                                        {
                                            glog("\tmalformed CLK_OE SET\n");
                                            return false;
                                        }   
                                        let mut enabled : bool = false;
                                        if input[1]!=b'0'
                                        {
                                            enabled=true;
                                        }
                                        bmp::bmp_platform_target_clk_output_enable(enabled);
                                        rpc_reply(rpc_commands::RPC_RESP_OK, 0);  
                                        return true;
                                    },
        
        rpc_commands::RPC_FREQ_SET => {   rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);    return true;},
        rpc_commands::RPC_FREQ_GET => {   rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);    return true;},
        rpc_commands::RPC_PWR_SET =>  {   rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);    return true;},
        rpc_commands::RPC_PWR_GET =>  {   rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);    return true;},
        _ => (),
        // b'A' REMOTE_START  => {},     remote_respond_string(REMOTE_RESP_OK, PLATFORM_IDENT "" FIRMWARE_VERSION);
    };
    rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);
    return true;
}
/*
 */

fn nbTick( pin: &[u8]) -> u32
{
    if pin.len()<2
    {
        glog("!!!! Size parm in swd SEQ too short\n");
        return 0;
    }
    crate::parsing_util::ascii_octet_to_hex(pin[0],pin[1]) as u32
}
/*

 */
#[no_mangle]
fn rpc_swdp_packet(input : &[u8]) -> bool
{
    glog("\tswd:\n");
    match input[0]
    {
        rpc_commands::RPC_INIT        => { 
                                    bmp::rpc_init_swd();  
                                    rpc_reply(rpc_commands::RPC_RESP_OK, 0);
                                    return true;
                                },
        rpc_commands::RPC_IN_PAR      => {
                                    let tick = nbTick(&input[1..=2]);
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let mut value : u32 =0;
                                    let mut parity : bool = false;
                                    match bmp::bmp_rpc_swd_in_par(&mut value, &mut parity, tick )
                                    {
                                        true => 
                                                    rpc_reply32(
                                                        match parity
                                                        {
                                                            true => rpc_commands::RPC_RESP_PARERR,
                                                            false => rpc_commands::RPC_RESP_OK,
                                                        }, value),
                                        false =>  rpc_reply(rpc_commands::RPC_RESP_ERR,0),
                                    };
                                    return true;
                                },
        rpc_commands::RPC_IN          => {
                                    let tick = nbTick(&input[1..=2]);
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let mut value : u32 =0;
                                    match bmp::bmp_rpc_swd_in(&mut value, tick  )
                                    {
                                        true  =>  rpc_reply32(rpc_commands::RPC_RESP_OK, value),
                                        false =>  rpc_reply32(rpc_commands::RPC_RESP_ERR, value),                                        
                                    };
                                    return true;
                                },
        rpc_commands::RPC_OUT_PAR     => {
                                    let tick = nbTick(&input[1..=2]);
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let param = crate::parsing_util::u8s_string_to_u32(&input[3..]);
                                    bmp::bmp_rpc_swd_out_par(param,  tick  );
                                    rpc_reply(rpc_commands::RPC_RESP_OK, 0);
                                    return true;
                                },
        rpc_commands::RPC_OUT    => {
                                    let tick = nbTick(&input[1..=2]);
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    // total should be 1 (cmd) + 2 (size) + 2*x
                                    if input.len()<5
                                    {
                                        glog("RPC_out wrong len\n");
                                        return false;
                                    }


                                    let param = crate::parsing_util::u8s_string_to_u32(&input[3..]);
                                    bmp::bmp_rpc_swd_out(param,  tick  );
                                    rpc_reply(rpc_commands::RPC_RESP_OK, 0);
                                    return true;
                            },
        _                   => (),
    };
    glog("unmanaged swdp packet\n");
    false
}
/*
 */
fn rpc_jtag_packet(input : &[u8]) -> bool
{
    glog("jtag packet\n");
    false
}

/*
 */
#[no_mangle]
fn rpc_wrapper(input : &[u8]) -> bool
{
    if input.len() < 2 // unlikely...?
    {
        glog("**short rpc\n");
        return false;
    }
    glog1("rpc call (",input.len());glog(")\n");
    if input.len()>2
    {
        glog1("\tClass : ",input[0]);
        glog1("Cmd : ",input[1]);
        glog("\n");
    }
    return match input[0]
    {
        rpc_commands::RPC_JTAG_PACKET  => rpc_jtag_packet(&input[1..]),
        rpc_commands::RPC_SWDP_PACKET  => rpc_swdp_packet(&input[1..]),
        rpc_commands::RPC_GEN_PACKET   => rpc_gen_packet(&input[1..]),
        rpc_commands::RPC_HL_PACKET    => rpc_hl_packet(&input[1..]),
        _ => {glog("wrong RPC header\n");return false;}
    };    
}

/**
 * 
 */
pub fn rpc(input : &[u8]) -> bool
{
   if !rpc_wrapper(input)
   {
        rpc_reply(rpc_commands::RPC_RESP_ERR, rpc_commands::RPC_REMOTE_ERROR_UNRECOGNISED);
   }
   true
}
/*
 #include "general.h"
 #include "remote.h"
 #include "gdb_packet.h"
 #include "jtagtap.h"
 #include "swd.h"
 #include "gdb_if.h"
 #include "version.h"
 #include "exception.h"
 #include <stdarg.h>
 #include "target/adiv5.h"
 #include "target.h"
 #include "hex_utils.h"
 
 #define NTOH(x)    (((x) <= 9) ? (x) + '0' : 'a' + (x)-10)
 #define HTON(x)    (((x) <= '9') ? (x) - '0' : ((TOUPPER(x)) - 'A' + 10))
 #define TOUPPER(x) ((((x) >= 'a') && ((x) <= 'z')) ? ((x) - ('a' - 'A')) : (x))
 #define ISHEX(x)   ((((x) >= '0') && ((x) <= '9')) || (((x) >= 'A') && ((x) <= 'F')) || (((x) >= 'a') && ((x) <= 'f')))
 
 /@ Return numeric version of string, until illegal hex digit, or max @/
 uint64_t remotehston(const uint32_t max, const char *const str)
 {
     uint64_t ret = 0;
     for (size_t i = 0; i < max; ++i) {
         const char value = str[i];
         if (!ISHEX(value))
             return ret;
         ret = (ret << 4U) | HTON(value);
     }
     return ret;
 }
 
 #if PC_HOSTED == 0
 static void remote_send_buf(uint8_t *buffer, size_t len)
 {
     uint8_t *p = buffer;
     char hex[2];
     do {
         hexify(hex, (const void *)p++, 1);
 
         gdb_if_putchar(hex[0], 0);
         gdb_if_putchar(hex[1], 0);
 
     } while (p < (buffer + len));
 }
 
 static void remote_respond_buf(char respCode, uint8_t *buffer, size_t len)
 {
     gdb_if_putchar(REMOTE_RESP, 0);
     gdb_if_putchar(respCode, 0);
 
     remote_send_buf(buffer, len);
 
     gdb_if_putchar(REMOTE_EOM, 1);
 }
 
 /@ Send response to far end @/
 static void remote_respond(char respCode, uint64_t param)
 {
     char buf[35]; /@Response, code, EOM and 2*16 hex nibbles@/
     char *p = buf;
 
     gdb_if_putchar(REMOTE_RESP, 0);
     gdb_if_putchar(respCode, 0);
 
     do {
         *p++ = NTOH(param & 0x0fU);
         param >>= 4U;
     } while (param);
 
     /@ At this point the number to print is the buf, but backwards, so spool it out @/
     do {
         gdb_if_putchar(*--p, 0);
     } while (p > buf);
     gdb_if_putchar(REMOTE_EOM, 1);
 }
 
 static void remote_respond_string(char respCode, const char *s)
 /@ Send response to far end @/
 {
     gdb_if_putchar(REMOTE_RESP, 0);
     gdb_if_putchar(respCode, 0);
     while (*s) {
         /@ Just clobber illegal characters so they don't disturb the protocol @/
         if ((*s == '$') || (*s == REMOTE_SOM) || (*s == REMOTE_EOM))
             gdb_if_putchar(' ', 0);
         else
             gdb_if_putchar(*s, 0);
         s++;
     }
     gdb_if_putchar(REMOTE_EOM, 1);
 }
 
 
 static void remote_packet_process_general(unsigned i, char *packet)
 {
     (void)i;
     uint32_t freq;
     switch (packet[1]) {
     case REMOTE_VOLTAGE:
         remote_respond_string(REMOTE_RESP_OK, platform_target_voltage());
         break;
 
     case REMOTE_NRST_SET:
         platform_nrst_set_val(packet[2] == '1');
         remote_respond(REMOTE_RESP_OK, 0);
         break;
 
     case REMOTE_NRST_GET:
         remote_respond(REMOTE_RESP_OK, platform_nrst_get_val());
         break;
     case REMOTE_FREQ_SET:
         platform_max_frequency_set(remotehston(8, packet + 2));
         remote_respond(REMOTE_RESP_OK, 0);
         break;
     case REMOTE_FREQ_GET:
         freq = platform_max_frequency_get();
         remote_respond_buf(REMOTE_RESP_OK, (uint8_t *)&freq, 4);
         break;
 
     case REMOTE_PWR_SET:
 #ifdef PLATFORM_HAS_POWER_SWITCH
         if (packet[2] == '1' && !platform_target_get_power() &&
             platform_target_voltage_sense() > POWER_CONFLICT_THRESHOLD) {
             /@ want to enable target power, but voltage > 0.5V sensed
              * on the pin -> cancel
              @/
             remote_respond(REMOTE_RESP_ERR, 0);
         } else {
             platform_target_set_power(packet[2] == '1');
             remote_respond(REMOTE_RESP_OK, 0);
         }
 #else
         remote_respond(REMOTE_RESP_NOTSUP, 0);
 #endif
         break;
 
     case REMOTE_PWR_GET:
 #ifdef PLATFORM_HAS_POWER_SWITCH
         remote_respond(REMOTE_RESP_OK, platform_target_get_power());
 #else
         remote_respond(REMOTE_RESP_NOTSUP, 0);
 #endif
         break;
 
 #if !defined(BOARD_IDENT) && defined(BOARD_IDENT)
 #define PLATFORM_IDENT() BOARD_IDENT
 #endif
     case REMOTE_START:
 #if defined(ENABLE_DEBUG) && defined(PLATFORM_HAS_DEBUG)
         //debug_bmp = true;
 #endif
         remote_respond_string(REMOTE_RESP_OK, PLATFORM_IDENT "" FIRMWARE_VERSION);
         break;
 
     case REMOTE_TARGET_CLK_OE:
         platform_target_clk_output_enable(packet[2] != '0');
         remote_respond(REMOTE_RESP_OK, 0);
         break;
 
     default:
         remote_respond(REMOTE_RESP_ERR, REMOTE_ERROR_UNRECOGNISED);
         break;
     }
 }
 
 static void remote_packet_process_high_level(unsigned i, char *packet)
 
 {
     (void)i;
     SET_IDLE_STATE(0);
 
     adiv5_access_port_s remote_ap;
     /@ Re-use packet buffer. Align to DWORD! @/
     void *src = (void *)(((uint32_t)packet + 7U) & ~7U);
     char index = packet[1];
     if (index == REMOTE_HL_CHECK) {
         remote_respond(REMOTE_RESP_OK, REMOTE_HL_VERSION);
         return;
     }
     packet += 2;
     remote_dp.dp_jd_index = remotehston(2, packet);
     packet += 2;
     remote_ap.apsel = remotehston(2, packet);
     remote_ap.dp = &remote_dp;
     switch (index) {
     case REMOTE_DP_READ: /@ Hd = Read from DP register @/
         packet += 2;
         uint16_t addr16 = remotehston(4, packet);
         uint32_t data = adiv5_dp_read(&remote_dp, addr16);
         remote_respond_buf(REMOTE_RESP_OK, (uint8_t *)&data, 4);
         break;
     case REMOTE_LOW_ACCESS: /@ HL = Low level access @/
         packet += 2;
         addr16 = remotehston(4, packet);
         packet += 4;
         uint32_t value = remotehston(8, packet);
         data = remote_dp.low_access(&remote_dp, remote_ap.apsel, addr16, value);
         remote_respond_buf(REMOTE_RESP_OK, (uint8_t *)&data, 4);
         break;
     case REMOTE_AP_READ: /@ Ha = Read from AP register@/
         packet += 2;
         addr16 = remotehston(4, packet);
         data = adiv5_ap_read(&remote_ap, addr16);
         remote_respond_buf(REMOTE_RESP_OK, (uint8_t *)&data, 4);
         break;
     case REMOTE_AP_WRITE: /@ Ha = Write to AP register@/
         packet += 2;
         addr16 = remotehston(4, packet);
         packet += 4;
         value = remotehston(8, packet);
         adiv5_ap_write(&remote_ap, addr16, value);
         remote_respond(REMOTE_RESP_OK, 0);
         break;
     case REMOTE_AP_MEM_READ: /@ HM = Read from Mem and set csw @/
         packet += 2;
         remote_ap.csw = remotehston(8, packet);
         packet += 6;
         /@fall through@/
     case REMOTE_MEM_READ: /@ Hh = Read from Mem @/
         packet += 2;
         uint32_t address = remotehston(8, packet);
         packet += 8;
         uint32_t count = remotehston(8, packet);
         packet += 8;
         adiv5_mem_read(&remote_ap, src, address, count);
         if (remote_ap.dp->fault == 0) {
             remote_respond_buf(REMOTE_RESP_OK, src, count);
             break;
         }
         remote_respond(REMOTE_RESP_ERR, 0);
         remote_ap.dp->fault = 0;
         break;
     case REMOTE_AP_MEM_WRITE_SIZED: /@ Hm = Write to memory and set csw @/
         packet += 2;
         remote_ap.csw = remotehston(8, packet);
         packet += 6;
         /@fall through@/
     case REMOTE_MEM_WRITE_SIZED: /@ HH = Write to memory@/
         packet += 2;
         align_e align = remotehston(2, packet);
         packet += 2;
         uint32_t dest = remotehston(8, packet);
         packet += 8;
         size_t len = remotehston(8, packet);
         packet += 8;
         if (len & ((1U << align) - 1U)) {
             /@ len  and align do not fit@/
             remote_respond(REMOTE_RESP_ERR, 0);
             break;
         }
         /@ Read as stream of hexified bytes@/
         unhexify(src, packet, len);
         adiv5_mem_write_sized(&remote_ap, dest, src, len, align);
         if (remote_ap.dp->fault) {
             /@ Errors handles on hosted side.@/
             remote_respond(REMOTE_RESP_ERR, 0);
             remote_ap.dp->fault = 0;
             break;
         }
         remote_respond(REMOTE_RESP_OK, 0);
         break;
     default:
         remote_respond(REMOTE_RESP_ERR, REMOTE_ERROR_UNRECOGNISED);
         break;
     }
     SET_IDLE_STATE(1);
 }
 
 void remote_packet_process(unsigned i, char *packet)
 {
     switch (packet[0]) {
     case REMOTE_SWDP_PACKET:
         remote_packet_process_swd(i, packet);
         break;
 
     case REMOTE_JTAG_PACKET:
         remote_packet_process_jtag(i, packet);
         break;
 
     case REMOTE_GEN_PACKET:
         remote_packet_process_general(i, packet);
         break;
 
     case REMOTE_HL_PACKET:
         remote_packet_process_high_level(i, packet);
         break;
 
     default: /@ Oh dear, unrecognised, return an error @/
         remote_respond(REMOTE_RESP_ERR, REMOTE_ERROR_UNRECOGNISED);
         break;
     }
 }
*/ 

// EOF

