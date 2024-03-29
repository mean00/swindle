/**
 * This handles the low level RPC as used when BMP is running in hosted mode
 * It is a parralel path to the normal gdb command and is BMP specific
 */
use crate::bmp;
use crate::encoder::*;
use crate::rpc::rpc_commands;

use crate::bmplogger::*;
use crate::crc::do_local_crc32;
use crate::rpc::rpc_parser::rpc_parameter_parser;
//------------------------------
crate::setup_log!(false);
use crate::{bmplog, bmpwarning};
//-------------------------------
/**
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
#[no_mangle]
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
        rpc_commands::RPC_SWINDLE_PACKET => rpc_swindle_packet(&mut parser),
        rpc_commands::RPC_RV_PACKET => rpc_rv_packet(&mut parser),
        rpc_commands::RPC_ADIV5_PACKET => rpc_adiv5_packet(&mut parser),
        rpc_commands::RPC_JTAG_PACKET => rpc_jtag_packet(&mut parser),
        rpc_commands::RPC_SWDP_PACKET => rpc_swdp_packet(&mut parser),
        rpc_commands::RPC_GEN_PACKET => rpc_gen_packet(&mut parser),
        rpc_commands::RPC_HL_PACKET => rpc_hl_packet_v3(&mut parser),
        _ => {
            bmplog!("wrong RPC header\n");
            false
        }
    }
}
/**
 *
 */
fn rpc_reply(code: u8, subcode: u8) {
    let mut reply: [u8; 5] = [0, 0, 0, 0, 0];

    let ascii = crate::parsing_util::u8_to_ascii(subcode);

    reply[0] = rpc_commands::RPC_REMOTE_RESP;
    reply[1] = code;
    reply[2] = ascii[0];
    reply[3] = ascii[1];
    reply[4] = crate::packet_symbols::RPC_END;
    rpc_message_out(&reply);
}
/**
 *
 */
fn rpc_reply32(code: u8, subcode: u32) {
    let mut reply: [u8; 11] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    reply[0] = rpc_commands::RPC_REMOTE_RESP;
    reply[1] = code;
    for i in 0..4 {
        let ascii = crate::parsing_util::u8_to_ascii(((subcode >> (8 * (3 - i))) & 0xff) as u8);
        reply[2 + i * 2] = ascii[0];
        reply[3 + i * 2] = ascii[1];
    }
    reply[10] = crate::packet_symbols::RPC_END;
    rpc_message_out(&reply);
}
/**
 *
 */
fn rpc_reply32_le(code: u8, subcode: u32) {
    let mut reply: [u8; 11] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    reply[0] = rpc_commands::RPC_REMOTE_RESP;
    reply[1] = code;
    let mut sc = subcode;
    for i in 0..4 {
        let ascii = crate::parsing_util::u8_to_ascii((sc & 0xff) as u8);
        sc >>= 8;
        reply[2 + i * 2] = ascii[0];
        reply[3 + i * 2] = ascii[1];
    }
    reply[10] = crate::packet_symbols::RPC_END;
    rpc_message_out(&reply);
}

/**
 *
 */
#[no_mangle]
fn rpc_reply_string(code: u8, s: &[u8]) {
    rpc_message_out_no_flush(&[rpc_commands::RPC_REMOTE_RESP, code]);
    rpc_message_out_no_flush(s);
    rpc_message_out(&[crate::packet_symbols::RPC_END]);
}
/**
 *
 */
fn rpc_reply_hex_string(code: u8, s: &[u8]) {
    rpc_message_out_no_flush(&[rpc_commands::RPC_REMOTE_RESP, code]);
    encoder::hexify_and_raw_send(s);
    rpc_message_out(&[crate::packet_symbols::RPC_END]);
}
/**
 *
 */
fn reply_adiv5_32(fault: i32, value: u32) {
    if fault != 0 {
        bmplog!("\tAdiv error   : 0x{:x}\n", fault as u32);
        rpc_reply32_le(
            rpc_commands::RPC_RESP_ERR,
            ((fault as u32) << 8) + (rpc_commands::RPC_ERROR_FAULT as u32),
        );
    } else {
        rpc_reply32_le(rpc_commands::RPC_RESP_OK, value);
    }
}
/**
 *
 */
fn reply_rv_32(ok: bool, value: u32) {
    if !ok {
        bmplog!("\rv error   \n");
        rpc_reply32_le(
            rpc_commands::RPC_RESP_ERR,
            ((1_u32) << 8) + (rpc_commands::RPC_ERROR_FAULT as u32),
        );
    } else {
        rpc_reply32_le(rpc_commands::RPC_RESP_OK, value);
    }
}
/**
 *
 */
fn reply_adiv5_block(fault: i32, buffer: &[u8]) {
    if fault != 0 {
        bmplog!("\reply_adiv5_block error   : 0x{:x}", fault as u32);
        rpc_reply32_le(
            rpc_commands::RPC_RESP_ERR,
            ((fault as u32) << 8) + (rpc_commands::RPC_ERROR_FAULT as u32),
        );
    } else {
        rpc_reply_hex_string(rpc_commands::RPC_RESP_OK, buffer);
    }
}
/*
 */
fn rpc_hl_packet_v3(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\thl packet\n");
    let cmd = parser.next_cmd();
    if cmd == rpc_commands::RPC_HL_CHECK {
        bmplog!("\t\tget version\n");
        const force_version: u8 = 3; //rpc_commands::RPC_HL_VERSION ; // Force version X
        rpc_reply(rpc_commands::RPC_RESP_OK, force_version); // Force version X
        return true;
    }

    // the follow up is
    // - index dp (2)
    // - ap.apsel (2)
    // 0   1    2    3    4    5    6    7    8
    // CMD INDEXX    AP_SEL    ADDREEEEEEEEESSS
    // [...]
    let device_index: u32 = parser.next_u8();
    let ap_selection: u32 = parser.next_u8(); //3 4

    match cmd {
        rpc_commands::RPC_RAW_ACCESS_V3 => {
            // index(u8) ap_sel(u8) addr(u16_be) data(u32_be)
            //--------------------------------------------
            let address: u32 = parser.next_u16_be();
            let mut value: u32 = 0;
            if !parser.end().is_empty() {
                value = parser.next_u32_be();
            }
            let fault: i32;
            bmplog!("\t\t LOW_ACCESS: {}\n", address);
            bmplog!("\t\t device_index {} ", device_index);
            bmplog!(" ap_selection at {}", ap_selection);
            bmplog!("\t\taddress :0x{:x}\n", address);
            bmplog!("\t\tvalue :{}\n", value);
            let outvalue: u32;
            (fault, outvalue) =
                bmp::bmp_adiv5_full_dp_low_level(device_index, ap_selection, address as u16, value);
            bmplog!("\t\toutvalue : 0x{:x} \n", outvalue);
            reply_adiv5_32(fault, outvalue);
            return true;
        }
        rpc_commands::RPC_AP_READ => {
            // x
            // index(U8)  ap_sel(u8) addr(u16_be)
            let address: u32 = parser.next_u16_be();
            let value = bmp::bmp_adiv5_ap_read(device_index, ap_selection, address);
            bmplog!("\t\t AP_READ addr:{}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, value);
            return true;
        }
        rpc_commands::RPC_DP_READ => {
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
            reply_adiv5_32(fault, value);
            return true;
        }

        rpc_commands::RPC_AP_WRITE => {
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

        rpc_commands::RPC_MEM_READ => {
            //M xx
            // index(u8) ap_sel(u8) csw(u32_be) address(u32_be) count(u32)
            //M0000a3000040e000edfc00000004
            let csw1: u32 = parser.next_u32_be();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t MEM READ CSW : 0x{:x} \n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            if length > 1024 {
                bmpwarning!("RPC_MEM_READ BLOCK TOO BIG len  :{}\n", length);
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
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
            reply_adiv5_block(fault, &buffer[0..l]); // xxx To check!
            return true;
        } // M
        rpc_commands::RPC_MEM_WRITE => {
            // m0000a300004002e000edfc0000000401040001
            //  // index(u8) ap_sel(u8) csw(u32_be) align(u8) addr(u32_be) count(u32) then data...
            let csw1: u32 = parser.next_u32_be();
            let align: u32 = parser.next_u8();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t RPC_MEM_WRITE CSW :0x{:x}\n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            bmplog!("\t\t align  :{}\n", align);
            if length > 1024 {
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
                return true;
            }
            let mut buffer: [u8; 1024] = [0; 1024];
            let decoded = crate::parsing_util::u8_hex_string_to_u8s(parser.end(), &mut buffer);
            let fault: i32 =
                bmp::bmp_adiv5_mem_write(device_index, ap_selection, csw1, address, align, decoded);
            reply_adiv5_32(fault, 0);
            return true;
        } // m

        _ => (),
    };

    bmplog!("\t\t Other HL cmd \n");
    //panic!();
    bmpwarning!("\tunknown hl packet 0x{:x}\n", cmd);
    rpc_reply(
        rpc_commands::RPC_RESP_ERR,
        rpc_commands::RPC_REMOTE_ERROR_UNRECOGNISED,
    );
    true
}
/*
 */
fn rpc_hl_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\thl packet\n");
    let cmd = parser.next_cmd();
    if cmd == rpc_commands::RPC_HL_CHECK {
        bmplog!("\t\tget version\n");
        const force_version: u8 = 3; //rpc_commands::RPC_HL_VERSION ; // Force version X
        rpc_reply(rpc_commands::RPC_RESP_OK, force_version); // Force version X
        return true;
    }

    // the follow up is
    // - index dp (2)
    // - ap.apsel (2)
    // 0   1    2    3    4    5    6    7    8
    // CMD INDEXX    AP_SEL    ADDREEEEEEEEESSS
    // [...]
    let device_index: u32 = parser.next_u8();
    let ap_selection: u32 = parser.next_u8(); //3 4

    match cmd {
        rpc_commands::RPC_DP_READ => {
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
            reply_adiv5_32(fault, value);
            return true;
        }
        rpc_commands::RPC_LOW_ACCESS => {
            // 0 12 34 56 78 90 12 34 56
            // L 00 00*00-04*50-00-00-00
            let address: u32 = parser.next_u16_be();
            let mut value: u32 = 0;
            if !parser.end().is_empty() {
                value = parser.next_u32_be();
            }
            let fault: i32;
            bmplog!("\t\t HL LOW_ACCESS: {}\n", address);
            bmplog!("\t\t device_index {} ", device_index);
            bmplog!(" ap_selection at {}", ap_selection);
            bmplog!("\t\taddress :0x{:x}\n", address);
            bmplog!("\t\tvalue :{}\n", value);
            let outvalue: u32;
            (fault, outvalue) =
                bmp::bmp_adiv5_full_dp_low_level(device_index, ap_selection, address as u16, value);
            bmplog!("\t\toutvalue : 0x{:x} \n", outvalue);
            reply_adiv5_32(fault, outvalue);
            return true;
        }
        rpc_commands::RPC_AP_READ =>
        //'a
        {
            let address: u32 = parser.next_u16_be();
            let value = bmp::bmp_adiv5_ap_read(device_index, ap_selection, address);
            bmplog!("\t\t AP_READ addr:{}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, value);
            return true;
        }
        rpc_commands::RPC_AP_WRITE =>
        // 'A
        {
            let address: u32 = parser.next_u16_be();
            let value: u32 = parser.next_u16_be();
            bmp::bmp_adiv5_ap_write(device_index, ap_selection, address, value);
            bmplog!("\t\t AP_WRITE addr:0x{:x}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, 0);
            return true;
        }

        rpc_commands::RPC_MEM_READ => {
            //M0000a3000040e000edfc00000004
            let csw1: u32 = parser.next_u32_be();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t MEM READ CSW : 0x{:x} \n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            if length > 1024 {
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
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
            reply_adiv5_block(fault, &buffer[0..l]);
            return true;
        } // M
        rpc_commands::RPC_MEM_WRITE => {
            // m0000a300004002e000edfc0000000401040001
            let csw1: u32 = parser.next_u32_be();
            let align: u32 = parser.next_u8();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t RPC_MEM_WRITE CSW :0x{:x}\n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            bmplog!("\t\t align  :{}\n", align);
            if length > 1024 {
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
                return true;
            }
            let mut buffer: [u8; 1024] = [0; 1024];
            let decoded = crate::parsing_util::u8_hex_string_to_u8s(parser.end(), &mut buffer);
            let fault: i32 =
                bmp::bmp_adiv5_mem_write(device_index, ap_selection, csw1, address, align, decoded);
            reply_adiv5_32(fault, 0);
            return true;
        } // m

        _ => (),
    };

    bmplog!("\t\t Other HL cmd \n");
    //panic!();
    bmpwarning!("\tunknown hl packet {}\n", cmd);
    rpc_reply(
        rpc_commands::RPC_RESP_ERR,
        rpc_commands::RPC_REMOTE_ERROR_UNRECOGNISED,
    );
    true
}
/*
 */
#[no_mangle]
fn rpc_gen_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\tgen packet\n");

    match parser.next_cmd() {
        rpc_commands::RPC_START => {
            bmplog!("rpc start session\n");
            rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"LNBMP");
            return true;
        }
        rpc_commands::RPC_VOLTAGE => {
            bmplog!("rpc voltage\n");
            rpc_reply_string(rpc_commands::RPC_REMOTE_RESP_OK, b"????");
            return true;
        }
        rpc_commands::RPC_NRST_SET => {
            bmplog!("rpc rst set\n");
            let mut enable: bool = false;
            if parser.next_cmd() != b'0' {
                enable = true;
            }
            bmp::bmp_platform_nrst_set_val(enable);
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
            return true;
        }
        rpc_commands::RPC_NRST_GET => {
            bmplog!("rpc rst get\n");
            let enabled: u8 = bmp::bmp_platform_nrst_get_val() as u8;
            rpc_reply(rpc_commands::RPC_RESP_OK, enabled);
            return true;
        }
        rpc_commands::RPC_TARGET_CLK_OE => {
            bmplog!("\trpc OE clk\n");
            let mut enabled: bool = false;
            if parser.next_cmd() != b'0' {
                enabled = true;
            }
            bmp::bmp_platform_target_clk_output_enable(enabled);
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
            return true;
        }

        rpc_commands::RPC_FREQ_SET => {
            bmplog!("\trpcFreq SET\n");
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
            return true;
        }
        rpc_commands::RPC_FREQ_GET => {
            bmplog!("\trpcFreq GET\n");
            rpc_reply32_le(rpc_commands::RPC_RESP_OK, 4000); // hardcode 4 Mbis
            return true;
        }

        //        rpc_commands::RPC_FREQ_GET => {   rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);    return true;},
        rpc_commands::RPC_PWR_SET => {
            rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);
            return true;
        }
        rpc_commands::RPC_PWR_GET => {
            rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);
            return true;
        }
        _ => (),
        // b'A' REMOTE_START  => {},     remote_respond_string(REMOTE_RESP_OK, PLATFORM_IDENT "" FIRMWARE_VERSION);
    };
    rpc_reply(rpc_commands::RPC_RESP_NOTSUP, 0);
    true
}

/*

*/
#[no_mangle]
fn rpc_swdp_packet(parser: &mut rpc_parameter_parser) -> bool {
    bmplog!("\tswd:\n");
    let cmd = parser.next_cmd();
    match cmd {
        rpc_commands::RPC_INIT => {
            bmplog!("\tinit swd\n");
            bmp::rpc_init_swd();
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
            return true;
        }
        rpc_commands::RPC_IN_PAR => {
            let tick = parser.next_u8();
            bmplog!("\tIn_par bits  : {}", tick);
            bmplog!("\n");
            if tick == 0 {
                return false;
            }
            let mut value: u32 = 0;
            let mut parity: bool = false;
            match bmp::bmp_rpc_swd_in_par(&mut value, &mut parity, tick) {
                true => {
                    //bmplog!("\tIn_par value  : ",value); bmplog!("\n");
                    rpc_reply32(
                        match parity {
                            true => {
                                bmplog!("In: BAD PARITY\n");
                                rpc_commands::RPC_RESP_PARERR
                            }
                            false => {
                                bmplog!("\t\t value 0x{:x}\n", value);
                                rpc_commands::RPC_RESP_OK
                            }
                        },
                        value,
                    );
                }
                false => {
                    bmplog!("In: FAIL\n");
                    rpc_reply32(rpc_commands::RPC_RESP_ERR, value)
                }
            };
            return true;
        }
        rpc_commands::RPC_IN => {
            let tick = parser.next_u8();
            bmplog!("\tIn bits  : {}\n", tick);
            if tick == 0 {
                return false;
            }
            let mut value: u32 = 0;
            match bmp::bmp_rpc_swd_in(&mut value, tick) {
                true => {
                    bmplog!("\t\t value 0x{:x}\n", value);
                    rpc_reply32(rpc_commands::RPC_RESP_OK, value);
                }
                false => {
                    bmplog!("In: FAIL\n");
                    rpc_reply32(rpc_commands::RPC_RESP_ERR, value);
                }
            };
            return true;
        }
        rpc_commands::RPC_OUT_PAR => {
            let tick = parser.next_u8();
            bmplog!("\tOut_par bits  : {}", tick);
            if tick == 0 {
                return false;
            }
            let param = parser.next_u32_be();
            bmplog!("\tOut_par value  : 0x{:x}\n", param);

            bmp::bmp_rpc_swd_out_par(param, tick);
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
            return true;
        }
        rpc_commands::RPC_OUT => {
            let tick = parser.next_u8();
            if tick == 0 {
                return false;
            }
            bmplog!("\tOut bits  :{} \n", tick);
            // total should be 1 (cmd) + 2 (size) + 2*x

            let param = parser.next_u32_be();
            bmplog!("\tOut_par value  : 0x{:x}\n", param);

            bmp::bmp_rpc_swd_out(param, tick);
            rpc_reply(rpc_commands::RPC_RESP_OK, 0);
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

/**
 *
 */
#[no_mangle]
fn rpc_rv_packet(parser: &mut rpc_parameter_parser) -> bool {
    match parser.next_cmd() {
        rpc_commands::RPC_RV_RESET => {
            let success = bmp::bmp_rv_reset();
            reply_rv_32(success, 0);
            return true;
        }
        rpc_commands::RPC_RV_DM_READ => {
            let value: u32;
            let ok: bool;
            let address: u32 = parser.next_u32();
            (ok, value) = bmp::bmp_rv_read(address as u8);
            reply_rv_32(ok, value);
            return true;
        }
        rpc_commands::RPC_RV_DM_WRITE => {
            let address: u32 = parser.next_u32();
            let value: u32 = parser.next_u32();
            //bmpwarning!("RV_DM_WRITE adr = {:x} val = {:x}\n",address,value);
            let ok = bmp::bmp_rv_write(address as u8, value);
            reply_rv_32(ok, 0);
            return true;
        }
        _ => (),
    }
    bmplog!("**** unsupported rv packet*********\n");
    false
}
/**
 *
 */
fn rpc_adiv5_packet(parser: &mut rpc_parameter_parser) -> bool {
    let cmd = parser.next_cmd();
    let device_index: u32 = parser.next_u8();
    let ap_selection: u32 = parser.next_u8(); //3 4

    match cmd {
        rpc_commands::RPC_RAW_ACCESS_V3 => {
            // index(u8) ap_sel(u8) addr(u16_be) data(u32_be)
            //--------------------------------------------
            let address: u32 = parser.next_u16_be();
            let mut value: u32 = 0;
            if !parser.end().is_empty() {
                value = parser.next_u32_be();
            }
            let fault: i32;
            bmplog!("\t\t ADIV LOW_ACCESS: {}\n", address);
            bmplog!("\t\t device_index {} ", device_index);
            bmplog!(" ap_selection at {}", ap_selection);
            bmplog!("\t\taddress :0x{:x}\n", address);
            bmplog!("\t\tvalue :{}\n", value);
            let outvalue: u32;
            (fault, outvalue) =
                bmp::bmp_adiv5_full_dp_low_level(device_index, ap_selection, address as u16, value);
            bmplog!("\t\toutvalue : 0x{:x} \n", outvalue);
            reply_adiv5_32(fault, outvalue);
            return true;
        }
        rpc_commands::RPC_AP_READ => {
            // x
            // index(U8)  ap_sel(u8) addr(u16_be)
            let address: u32 = parser.next_u16_be();
            let value = bmp::bmp_adiv5_ap_read(device_index, ap_selection, address);
            bmplog!("\t\t AP_READ addr:{}\n", address);
            bmplog!("\t\t value 0x{:x} \n", value);
            reply_adiv5_32(0, value);
            return true;
        }
        rpc_commands::RPC_DP_READ => {
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
            reply_adiv5_32(fault, value);
            return true;
        }

        rpc_commands::RPC_AP_WRITE => {
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

        rpc_commands::RPC_MEM_READ_V3 => {
            //m xx
            // index(u8) ap_sel(u8) csw(u32_be) address(u32_be) count(u32)
            let csw1: u32 = parser.next_u32_be();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t MEM READ CSW : 0x{:x} \n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            if length > 1024 {
                bmpwarning!("RPC_MEM_READ BLOCK TOO BIG len  :{}\n", length);
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
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
            reply_adiv5_block(fault, &buffer[0..l]); // xxx To check!
            return true;
        } // M
        rpc_commands::RPC_MEM_WRITE_V3 => {
            // AM0000a300004002e000edfc0000000401040001
            // index(u8) ap_sel(u8) csw(u32_be) align(u8) addr(u32_be) count(u32) then data...
            let csw1: u32 = parser.next_u32_be();
            let align: u32 = parser.next_u8();
            let address: u32 = parser.next_u32_be();
            let length: u32 = parser.next_u32_be();
            bmplog!("\t\t RPC_MEM_WRITE CSW :0x{:x}\n", csw1);
            bmplog!("\t\t adr  :0x{:x}\n", address);
            bmplog!("\t\t len  :{}\n", length);
            bmplog!("\t\t align  :{}\n", align);
            if length > 1024 {
                rpc_reply(rpc_commands::RPC_REMOTE_RESP_PARERR, 0);
                return true;
            }
            let mut buffer: [u8; 1024] = [0; 1024];
            let decoded = crate::parsing_util::u8_hex_string_to_u8s(parser.end(), &mut buffer);
            let fault: i32 =
                bmp::bmp_adiv5_mem_write(device_index, ap_selection, csw1, address, align, decoded);
            reply_adiv5_32(fault, 0);
            return true;
        } // m

        _ => (),
    };
    bmplog!("**** FAILED : adiv5 packet*********\n");
    false
}

/**
 *
 */
pub fn rpc(input: &[u8]) -> bool {
    if !rpc_wrapper(input) {
        rpc_reply(
            rpc_commands::RPC_RESP_ERR,
            rpc_commands::RPC_REMOTE_ERROR_UNRECOGNISED,
        );
    }
    true
}
/**
 * \fn rpc_swindle_packet
 */
#[no_mangle]
fn rpc_swindle_packet(parser: &mut rpc_parameter_parser) -> bool {
    match parser.next_cmd() {
        rpc_commands::RPC_SWINDLE_CRC32 => {
            let address: u32 = parser.next_u32();
            let len: u32 = parser.next_u32();
            let (status, crc) = do_local_crc32(address, len);
            reply_rv_32(status, crc);
            return true;
        }
        _ => (),
    }
    false
}

// EOF
