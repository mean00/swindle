/*
 * This handles the low level RPC as used when BMP is running in hosted mode
 * It is a parralel path to the normal gdb command and is BMP specific
 */
use crate::bmp;
use crate::encoder::*;
pub mod rpc_parser;
pub mod rpc_reply;
use crate::bmp::{bmp_get_frequency, bmp_set_frequency};
use crate::crc::do_local_crc32;
use crate::parsing_util;
use crate::rpc_common::*;
use crate::rpc_target::rpc_reply::*;
use rpc_parser::rpc_parameter_parser;
//------------------------------
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};
//-------------------------------
/*
 *
 */
fn rpc_message_out(message: &[u8]) {
    encoder::raw_send_u8(message);
    encoder::flush();
}
fn rpc_message_out_no_flush(message: &[u8]) {
    encoder::raw_send_u8(message);
}

/*
 */
#[unsafe(no_mangle)]
fn rpc_wrapper(input: &[u8]) -> bool {
    if input.len() < 2
    // unlikely...?
    {
        bmplog!("**short rpc\n");
        return false;
    }
    bmplog!("rpc call ( {}", input.len());
    let as_string = unsafe { core::str::from_utf8_unchecked(input) };
    bmplog!(") <{}>\n", as_string);
    if input.len() > 2 {
        bmplog!("\tClass : {}", input[0]);
        bmplog!("Cmd : {}", input[1]);
        bmplog!("\n");
    }
    let mut parser = rpc_parameter_parser::new(input);
    match parser.next_cmd() {
        RPC_SWINDLE_PACKET => rpc_swindle_packet(&mut parser),
        RPC_RV_PACKET => rpc_rv_packet(&mut parser),
        RPC_ADIV5_PACKET => rpc_adiv5_packet(&mut parser),
        RPC_JTAG_PACKET => rpc_jtag_packet(&mut parser),
        RPC_SWDP_PACKET => rpc_swdp_packet(&mut parser),
        RPC_GEN_PACKET => rpc_gen_packet(&mut parser),
        RPC_HL_PACKET => rpc_hl_packet_v3(&mut parser),
        RPC_LNADIV_PACKET => rpc_lnadiv_packet(&mut parser),
        RPC_SPI_PACKET => {
            bmplog!("remote SPI packet, unsupported \n");
            false
        }
        _ => {
            bmplog!("wrong RPC header\n");
            false
        }
    }
}
/*
 *
 */
fn rpc_reply_ok(subcode: u8) {
    let mut reply = rpc_reply_encoder::new();
    reply.add_u8_wrapped(subcode);
    reply.end();
}
//
fn rpc_reply32_le_ok(subcode: u32) {
    let mut reply = rpc_reply_encoder::new();
    reply.add_u32_le_wrapped(subcode);
    reply.end();
}
fn rpc_reply_error(subcode: u8) {
    rpc_reply_encoder::simple_error(subcode);
}
fn rpc_reply_full_error(complex: &[u8]) {
    let mut reply = rpc_reply_encoder::new_error();
    for &i in complex {
        reply.add_u8_wrapped(i);
    }
    reply.end();
}

/*
 *
 */
#[unsafe(no_mangle)]
fn rpc_reply_string(s: &[u8]) {
    let mut reply = rpc_reply_encoder::new();
    reply.add_string(s);
    reply.end();
}
/*
 *
 */
fn rpc_reply_hex_string(code: u8, s: &[u8]) {
    rpc_message_out_no_flush(&[RPC_REMOTE_RESP, code]);
    encoder::hexify_and_raw_send(s);
    rpc_message_out(&[crate::packet_symbols::RPC_END]);
}
/*
 *
 */
fn reply_adiv5_32(fault: i32, value: u32) {
    if fault != 0 {
        bmplog!("\tAdiv error   : 0x{:x}\n", fault as u32);
        // Assume it is a FALT rpc_reply_error(fault as u8);
        let mut reply = rpc_reply_encoder::new_error();
        reply.add_u8_wrapped((fault & 0xff) as u8);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.add_u8_wrapped(RPC_ERROR_FAULT);
        reply.end();
    } else {
        rpc_reply32_le_ok(value);
    }
}
/*
 *
 */
fn rpc_reply_bool_32le(ok: bool, value: u32) {
    if !ok {
        bmplog!("\rv error   \n");
        rpc_reply_error(1_u8);
    } else {
        rpc_reply32_le_ok(value);
    }
}
/*
 *
 */
fn reply_adiv5_block(fault: i32, buffer: &[u8]) {
    if fault != 0 {
        bmplog!("\reply_adiv5_block error   : 0x{:x}", fault as u32);
        rpc_reply_error(fault as u8);
    } else {
        rpc_reply_hex_string(RPC_RESP_OK, buffer);
    }
}

#[unsafe(no_mangle)]
fn rpc_lnadiv_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\tlnadiv packet\n");
    match parser.next_cmd() {
        RPC_LNADIV_WRITE => {
            bmplog!("lnadiv_write\n");
            let address: u32 = parser.next_u32();
            let data: u32 = parser.next_u32();
            bmplog!("lnadiv_write at 0x{:x} data=0x{:x}\n", address, data);
            rpc_reply_bool_32le(bmp::bmp_adiv5_swd_write_no_check(address as u16, data), 0);
            return true;
        }
        RPC_LNADIV_READ => {
            let address: u32 = parser.next_u32();
            bmplog!("lnadiv_read at 0x{:x}\n", address);
            rpc_reply_bool_32le(true, bmp::bmp_adiv5_swd_read_no_check(address as u16));
            return true;
        }
        RPC_LNADIV_RAW_WRITE => {
            bmplog!("lnadiv_raw_write\n");
            let tick: u32 = parser.next_u32();
            let value: u32 = parser.next_u32();
            bmplog!("lnadiv_write {} bits  0x{:x}\n", tick, value);
            bmp::bmp_raw_swd_write(tick, value);
            rpc_reply_bool_32le(true, 0);
            return true;
        }
        RPC_LNADIV_LOWLEVEL => {
            bmplog!("lnadiv_lowlevel\n");
            let rnw: u32 = parser.next_u32();
            let addr: u32 = parser.next_u32();
            let value: u32 = parser.next_u32();
            let mut fault: u32 = 0;
            bmplog!(
                ">>lnadiv_lowlevel rw:{} addr 0x{:x} data=0x{:x}\n",
                rnw,
                addr,
                value
            );
            let response: u32 =
                bmp::bmp_adiv5_swd_raw_access(rnw as u8, addr as u16, value, &mut fault);
            let mut reply = rpc_reply_encoder::new();
            bmplog!("\treply value=0x{:x} fault={}\n", response, fault);
            reply.add_u32_le_wrapped(response);
            reply.add_u32_le_wrapped(fault);
            reply.end();
            return true;
        }
        _ => (),
    }
    false
}

/*
 */
#[unsafe(no_mangle)]
fn rpc_hl_packet_v3(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\thl packet\n");
    let cmd = parser.next_cmd();
    if cmd == RPC_HL_CHECK {
        bmplog!("\t\tget version\n");
        const force_version: u8 = 3; //RPC_HL_VERSION ; // Force version X
        let mut reply = rpc_reply_encoder::new();
        reply.add_u8_wrapped(force_version); // Force version X
        reply.end();
        return true;
    }
    if cmd == RPC_HL_ACCEL {
        bmplog!("\t\tget accel\n");
        let mut reply = rpc_reply_encoder::new();
        reply.add_u8_wrapped(1 << 0); // ADIV5
        reply.end();
        return true;
    }

    bmplog!("\t\t Other HL cmd \n");
    //panic!();
    bmpwarning!("\tunknown hl packet 0x{:x}\n", cmd);
    rpc_reply_error(RPC_REMOTE_ERROR_UNRECOGNISED);
    true
}
/*
 */
#[unsafe(no_mangle)]
fn rpc_gen_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\tgen packet\n");

    match parser.next_cmd() {
        RPC_START => {
            bmplog!("rpc start session\n");
            rpc_reply_string(b"LNBMP");
            return true;
        }
        RPC_VOLTAGE => {
            bmplog!("rpc voltage\n");
            rpc_reply_string(b"????");
            return true;
        }
        RPC_NRST_SET => {
            bmplog!("rpc rst set\n");
            let mut enable: bool = false;
            if parser.next_cmd() != b'0' {
                enable = true;
            }
            bmp::bmp_platform_nrst_set_val(enable);
            rpc_reply_ok(0);
            return true;
        }
        RPC_NRST_GET => {
            bmplog!("rpc rst get\n");
            let enabled: u8 = bmp::bmp_platform_nrst_get_val() as u8;
            rpc_reply_ok(enabled);
            return true;
        }
        RPC_TARGET_CLK_OE => {
            bmplog!("\trpc OE clk\n");
            let mut enabled: bool = false;
            if parser.next_cmd() != b'0' {
                enabled = true;
            }
            bmp::bmp_platform_target_clk_output_enable(enabled);
            rpc_reply_ok(0);
            return true;
        }

        RPC_FREQ_SET => {
            bmplog!("\trpcFreq SET\n");
            rpc_reply_ok(0);
            return true;
        }
        RPC_FREQ_GET => {
            bmplog!("\trpcFreq GET\n");
            rpc_reply32_le_ok(4000); // hardcode 4 Mbis
            return true;
        }

        //        RPC_FREQ_GET => {   rpc_reply(RPC_RESP_NOTSUP, 0);    return true;},
        RPC_PWR_SET => {
            rpc_reply_error(RPC_RESP_NOTSUP);
            return true;
        }
        RPC_PWR_GET => {
            rpc_reply_error(RPC_RESP_NOTSUP);
            return true;
        }
        _ => (),
        // b'A' REMOTE_START  => {},     remote_respond_string(REMOTE_RESP_OK, PLATFORM_IDENT "" FIRMWARE_VERSION);
    };
    rpc_reply_error(RPC_RESP_NOTSUP);
    true
}

/*

*/
#[unsafe(no_mangle)]
fn rpc_swdp_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\tswd:\n");
    let cmd = parser.next_cmd();
    match cmd {
        RPC_INIT => {
            bmplog!("\tinit swd\n");
            bmp::rpc_init_swd();
            rpc_reply_ok(0);
            return true;
        }
        _ => bmplog!("Unsupported SWP RPC command 0x{:x}\n", cmd),
    };
    bmplog!("unmanaged swdp packet\n");
    false
}
/*
 */
fn rpc_jtag_packet(_parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("jtag packet\n");
    false
}

/*
 *
 */
#[unsafe(no_mangle)]
fn rpc_rv_packet(parser: &mut rpc_parameter_parser) -> bool {
    match parser.next_cmd() {
        RPC_RV_RESET => {
            bmp::rv_dm_start();
            rpc_reply_bool_32le(true, 0);
            return true;
        }
        RPC_RV_DM_READ => {
            let value: u32;
            let ok: bool;
            let address: u32 = parser.next_u32();
            (ok, value) = bmp::bmp_rv_read(address as u8);
            rpc_reply_bool_32le(ok, value);
            return true;
        }
        RPC_RV_DM_WRITE => {
            let address: u32 = parser.next_u32();
            let value: u32 = parser.next_u32();
            //bmpwarning!("RV_DM_WRITE adr = {:x} val = {:x}\n",address,value);
            let ok = bmp::bmp_rv_write(address as u8, value);
            rpc_reply_bool_32le(ok, 0);
            return true;
        }
        _ => (),
    }
    bmplog!("**** unsupported rv packet*********\n");
    false
}
/*
 *
 */
fn rpc_adiv5_packet(parser: &mut rpc_parameter_parser) -> bool {
    let cmd = parser.next_cmd();
    bmp::bmp_clear_dp_fault();
    let device_index: u32 = parser.next_u8();
    let ap_selection: u32 = parser.next_u8(); //3 4

    match cmd {
        RPC_RAW_ACCESS_V3 => {
            // index(u8) ap_sel(u8) addr(u16_be) data(u32_be)
            //--------------------------------------------
            let address: u32 = parser.next_u16_be();
            let mut value: u32 = 0;
            if !parser.end().is_empty() {
                value = parser.next_u32_be();
            }
            let fault: i32;
            bmplog!("\t\t ADIV LOW_ACCESS: {}\n", address);
            bmplog!("\t\t device_index 0x{:x} ", device_index);
            bmplog!(" ap_selection at {}", ap_selection);
            bmplog!("\t\taddress :0x{:x}\n", address);
            bmplog!("\t\tvalue :0x{:x}\n", value);
            let outvalue: u32;
            (fault, outvalue) =
                bmp::bmp_adiv5_full_dp_low_level(device_index, ap_selection, address as u16, value);
            bmplog!("\t\toutvalue : 0x{:x} \n", outvalue);
            bmplog!("\t\t fault  :{}\n", fault);
            reply_adiv5_32(fault, outvalue);
            return true;
        }
        RPC_AP_READ => {
            // x
            // index(U8)  ap_sel(u8) addr(u16_be)
            let address: u32 = parser.next_u16_be();
            let value = bmp::bmp_adiv5_ap_read(device_index, ap_selection, address);
            bmplog!("\t\t AP_READ addr:0x{:x}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, value);
            return true;
        }
        RPC_DP_READ => {
            // x
            // index(u8)  ap_self = 0xff (u8) ADDR16
            let address: u32 = parser.next_u32_be();
            let value: u32;
            let fault: i32;

            bmplog!("\t\t DP_READ: 0x{:x}\n", address);
            bmplog!("\t\t device_index  {}", device_index);
            bmplog!(" ap_selection at {}", ap_selection);
            bmplog!(" dp_read at 0x{:x}", address);
            (fault, value) =
                bmp::bmp_adiv5_full_dp_read(device_index, ap_selection, address as u16);
            bmplog!("\t\tvalue :0x{:x}\n", value);
            bmplog!("\t\t fault  :{}\n", fault);
            reply_adiv5_32(fault, value);
            return true;
        }

        RPC_AP_WRITE => {
            // 'A xx
            // index (u8) ap_sel(u8) addr(u16_be) data(u32_be)
            let address: u32 = parser.next_u16_be();
            let value: u32 = parser.next_u32_be();
            bmp::bmp_adiv5_ap_write(device_index, ap_selection, address, value);
            bmplog!("\t\t AP_WRITE addr:0x{:x}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, 0);
            return true;
        }

        RPC_MEM_READ_V3 => {
            //m xx
            // index(u8) ap_sel(u8) csw(u32_be) address(u32_be) count(u32)
            let csw1: u32 = parser.next_u32_be();
            // parser.next_u32_be(); // 64 bits address, drop the MSB
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t MEM READ CSW : 0x{:x} \n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            if length > 1024 {
                bmpwarning!("RPC_MEM_READ BLOCK TOO BIG len  :{}\n", length);
                rpc_reply_error(RPC_REMOTE_RESP_PARERR);
                return true;
            }
            let mut buffer: [u8; 1024] = [0; 1024];
            let l: usize = length as usize;
            let fault: i32 = bmp::bmp_adiv5_mem_read(
                device_index,
                ap_selection,
                csw1,
                address,
                &mut buffer[0..l],
            );
            bmplog!("\t\t fault  :{}\n", fault);
            reply_adiv5_block(fault, &buffer[0..l]); // xxx To check!
            return true;
        } // M
        RPC_MEM_WRITE_V3 => {
            // AM0000a300004002e000edfc0000000401040001
            // index(u8) ap_sel(u8) csw(u32_be) align(u8) addr(u32_be) count(u32) then data...
            let csw1: u32 = parser.next_u32_be();
            let align: u32 = parser.next_u8();
            //parser.next_u32_be(); // 64 bits address, drop the MSB
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t RPC_MEM_WRITE CSW :0x{:x}\n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            bmplog!("\t\t align  :{}\n", align);
            if length > 1024 {
                rpc_reply_error(RPC_REMOTE_RESP_PARERR);
                return true;
            }
            let mut buffer: [u8; 1024] = [0; 1024];
            let decoded = parsing_util::u8_hex_string_to_u8s(parser.end(), &mut buffer);
            let fault: i32 =
                bmp::bmp_adiv5_mem_write(device_index, ap_selection, csw1, address, align, decoded);
            bmplog!("\t\t fault  :{}\n", fault);
            reply_adiv5_32(fault, 0);
            return true;
        } // m

        _ => (),
    };
    bmplog!("**** FAILED : adiv5 packet*********\n");
    false
}

/*
 *
 */
pub fn rpc(input: &[u8]) -> bool {
    if !rpc_wrapper(input) {
        rpc_reply_error(RPC_REMOTE_ERROR_UNRECOGNISED);
    }
    true
}
/*
 * \fn rpc_swindle_packet
 */
#[unsafe(no_mangle)]
fn rpc_swindle_packet(parser: &mut rpc_parameter_parser) -> bool {
    match parser.next_cmd() {
        RPC_SWINDLE_CRC32 => {
            let address: u32 = parser.next_u32();
            let len: u32 = parser.next_u32();
            let (status, crc) = do_local_crc32(address, len);
            rpc_reply_bool_32le(status, crc);
            return true;
        }
        RPC_SWINDLE_GET_FQ => {
            let fq: u32 = bmp_get_frequency();
            rpc_reply_bool_32le(true, fq);
            return true;
        }
        RPC_SWINDLE_SET_FQ => {
            let fq: u32 = parser.next_u32();
            bmp_set_frequency(fq);
            rpc_reply_ok(0);
            return true;
        }
        _ => bmpwarning!("Unknown swindle packet\n"),
    }
    false
}

// EOF
