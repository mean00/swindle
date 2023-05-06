
/**
 * This handles the low level RPC as used when BMP is running in hosted mode
 * It is a parralel path to the normal gdb command and is BMP specific
 */
use alloc::vec;
use alloc::vec::Vec;
use crate::bmp;
use crate::lnlogger::{*};
use crate::encoder::{*};
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::commands::{CallbackType,exec_one,CommandTree};
use crate::glue::gdb_out_rs;
use numtoa::NumToA;
use crate::commands::rpc_commands;



crate::setup_log!(false);

//-------------------------------
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
    bmplog("\thl packet\n");
    let mut res : u8 = 0;
    if input[0]== rpc_commands::RPC_HL_CHECK
    {
        rpc_reply(rpc_commands::RPC_RESP_OK, rpc_commands::RPC_HL_VERSION);
        /*
        rpc_message_
        out(&[
        rpc_commands::RPC_REMOTE_RESP,
        rpc_commands::RPC_RESP_OK,
        rpc_commands::RPC_HL_VERSION,
        crate::packet_symbols::RPC_END]);
        */
        return true;
    }    
    // the follow up is 
    // - index dp (2)
    // - ap.apsel (2)
    // [...]
    match input[0]
    {        
        RPC_DP_READ => (),
        RPC_LOW_ACCESS => (),
        RPC_AP_READ => (),
        RPC_AP_WRITE => (),
        RPC_AP_MEM_READ=> (),
        RPC_MEM_READ => (),
        RPC_MEM_WRITE_SIZED => (),
        RPC_AP_MEM_WRITE_SIZED=> (),
        _ => (),
    };

    bmpwarning("\tunknown hl packet\n",input[0]);
    rpc_reply(rpc_commands::RPC_RESP_ERR, rpc_commands::RPC_REMOTE_ERROR_UNRECOGNISED);
    true
}
/*
 */
#[no_mangle]
fn rpc_gen_packet(input : &[u8]) -> bool
{
    bmplog("\tgen packet\n");
    let mut res : u8 = 0;
    match input[0]
    {
        rpc_commands::RPC_START => {
                                        bmplog("rpc start session\n");
                                        rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"LNBMP");
                                        return true; 
                                    },
        rpc_commands::RPC_VOLTAGE => {
                                        bmplog("rpc voltage\n");
                                        rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"????"); 
                                        return true; 
                                    },
        rpc_commands::RPC_NRST_SET => {    
                                        bmplog("rpc rst set\n");
                                        if input.len() < 2
                                        {
                                            bmplog("\tmalformed NRST SET\n");
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
                                        bmplog("rpc rst get\n");
                                        let enabled : u8 = bmp::bmp_platform_nrst_get_val() as u8;
                                        rpc_reply(rpc_commands::RPC_RESP_OK, enabled);  
                                        return true;
                                    },
        rpc_commands::RPC_TARGET_CLK_OE => {       
                                        bmplog("\trpc OE clk\n");
                                        if input.len() < 2
                                        {
                                            bmplog("\tmalformed CLK_OE SET\n");
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
        bmplog("!!!! Size parm in swd SEQ too short\n");
        return 0;
    }
    crate::parsing_util::ascii_octet_to_hex(pin[0],pin[1]) as u32
}
/*

 */
#[no_mangle]
fn rpc_swdp_packet(input : &[u8]) -> bool
{
    bmplog("\tswd:\n");
    match input[0]
    {
        rpc_commands::RPC_INIT        => { 
                                    bmplog("\tinit swd\n");
                                    bmp::rpc_init_swd();  
                                    rpc_reply(rpc_commands::RPC_RESP_OK, 0);
                                    return true;
                                },
        rpc_commands::RPC_IN_PAR      => {                                    
                                    let tick = nbTick(&input[1..=2]);
                                    bmplog1("\tIn_par bits  : ",tick); bmplog("\n");
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let mut value : u32 =0;
                                    let mut parity : bool = false;
                                    match bmp::bmp_rpc_swd_in_par(&mut value, &mut parity, tick )
                                    {
                                        true => 
                                        {
                                                    bmplogx("\tIn_par value  : ",value); bmplog("\n");
                                                    rpc_reply32(
                                                        match parity
                                                        {
                                                            true => {bmplog("In: BAD PARITY\n");rpc_commands::RPC_RESP_PARERR},
                                                            false => {bmplog1("\t\t value",value);bmplog("\n");rpc_commands::RPC_RESP_OK},
                                                        }, value);},
                                        false =>  {bmplog("In: FAIL\n");rpc_reply32(rpc_commands::RPC_RESP_ERR,value)},
                                    };
                                    return true;
                                },
        rpc_commands::RPC_IN          => {
                                    let tick = nbTick(&input[1..=2]);
                                    bmplog1("\tIn bits  : ",tick); bmplog("\n");
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let mut value : u32 =0;
                                    match bmp::bmp_rpc_swd_in(&mut value, tick  )
                                    {
                                        true  =>  {bmplogx("\t\t value",value);bmplog("\n");rpc_reply32(rpc_commands::RPC_RESP_OK, value);},
                                        false =>   {bmplog("In: FAIL\n");rpc_reply32(rpc_commands::RPC_RESP_ERR, value);},
                                    };
                                    return true;
                                },
        rpc_commands::RPC_OUT_PAR     => {
                                    let tick = nbTick(&input[1..=2]);
                                    bmplog1("\tOut_par bits  : ",tick); 
                                    if tick == 0
                                    {
                                        return false;
                                    }
                                    let param = crate::parsing_util::u8s_string_to_u32(&input[3..]);
                                    bmplog1("\tOut_par value  : ",param); 
                                    bmplog("\n");

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
                                    bmplog1("\tOut bits  : ",tick); bmplog("\n");
                                    // total should be 1 (cmd) + 2 (size) + 2*x
                                    if input.len()<4
                                    {
                                        bmplog1("RPC_out wrong len:",input.len());bmplog("\n");
                                        return false;
                                    }


                                    let param = crate::parsing_util::u8s_string_to_u32(&input[3..]);
                                    bmplog1("\tOut_par value  : ",param); 
                                    bmplog("\n");

                                    bmp::bmp_rpc_swd_out(param,  tick  );
                                    rpc_reply(rpc_commands::RPC_RESP_OK, 0);
                                    return true;
                            },
        _                   => bmplog1("Unsupported SWP RPC command",input[0]),
    };
    bmplog("unmanaged swdp packet\n");
    false
}
/*
 */
fn rpc_jtag_packet(input : &[u8]) -> bool
{
    bmplog("jtag packet\n");
    false
}

/*
 */
#[no_mangle]
fn rpc_wrapper(input : &[u8]) -> bool
{
    if input.len() < 2 // unlikely...?
    {
        bmplog("**short rpc\n");
        return false;
    }
    bmplog1("rpc call (",input.len());bmplog(")\n");
    if input.len()>2
    {
        bmplog1("\tClass : ",input[0]);
        bmplog1("Cmd : ",input[1]);
        bmplog("\n");
    }
    return match input[0]
    {
        rpc_commands::RPC_JTAG_PACKET  => rpc_jtag_packet(&input[1..]),
        rpc_commands::RPC_SWDP_PACKET  => rpc_swdp_packet(&input[1..]),
        rpc_commands::RPC_GEN_PACKET   => rpc_gen_packet(&input[1..]),
        rpc_commands::RPC_HL_PACKET    => rpc_hl_packet(&input[1..]),
        _ => {bmplog("wrong RPC header\n");return false;}
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
 
*/ 

// EOF

