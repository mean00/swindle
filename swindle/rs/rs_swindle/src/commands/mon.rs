use crate::bmp;
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::freertos::enable_freertos;
use crate::parsing_util;
use crate::parsing_util::{ascii_hex_string_to_u8s, ascii_string_decimal_to_u32};
use alloc::vec::Vec;
//
//
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};
//
static mut targetCommandTree: Option<&[CommandTree]> = None;
static mut targetHelpTree: Option<&[HelpTree]> = None;
//
unsafe extern "C" {
    pub fn _Z17lnSoftSystemResetv();
}
/*
 *
 */
fn systemReset() {
    unsafe {
        _Z17lnSoftSystemResetv();
    }
}

//
const mon_command_tree: [CommandTree; 18] = [
    CommandTree {
        command: "redirect",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_redirect),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "enablereset",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_enable_reset),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "bmp",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_bmp_mon),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "boards",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_boards),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "frequency",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "fq",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "fos",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "freertos",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "help",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_help),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "os_info",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos_info),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "ram",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_ram),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "reboot",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_reboot),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "reset",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_target_reset),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "rvswdp_scan",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_rvswdp_scan),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "swdp_scan",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_swdp_scan),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "version",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_get_version),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "voltage",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_voltage),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "ws",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_ws),
        start_separator: " ",
        next_separator: " ",
    }, //
];
//
const help_tree: [HelpTree; 17] = [
    HelpTree {
        command: "help",
        help: "Display help.",
    },
    HelpTree {
        command: "bmp",
        help: "Forward the command to bmp mon command.\n\tExample : mon bmp mass_erase is the same as mon mass_erase on a bmp..",
    },
    HelpTree {
        command: "boards",
        help: "Display supported boards. This is set at build time.",
    },
    HelpTree {
        command: "ch32v3_obr",
        help: "Read/write the read protection status.",
    },
    HelpTree {
        command: "ch32v3_option_byte",
        help: "Read/write the user option byte on ch32v3 chip.\n\tThat changes the flash/ram split.\n\tUsual value : 256/64 -> 0x9f, 192/128 -> 0x1f.",
    },
    HelpTree {
        command: "fq or frequency",
        help: "set/get SWD frequency.",
    },
    HelpTree {
        command: "fos",
        help: "Enable FreeRTOS support.",
    },
    HelpTree {
        command: "os_info",
        help: "Dump FreeRTOS internal state.",
    },
    HelpTree {
        command: "ram",
        help: "Display stats about Ram usage.",
    },
    HelpTree {
        command: "reboot",
        help: "Reboot the debugger.",
    },
    HelpTree {
        command: "redirect 0|1",
        help: "Redirect log to usb2. mon redirect 1 to redirect, mon redirect 0 to not disable",
    },
    HelpTree {
        command: "reset",
        help: "Reset the target.",
    },
    HelpTree {
        command: "rvswdp_scan",
        help: "Probe WCH RISCV device(s).",
    },
    HelpTree {
        command: "swdp_scan",
        help: "Probe device(s) over SWD. You might want to increase wait state if it fails.",
    },
    HelpTree {
        command: "version",
        help: "Display version.",
    },
    HelpTree {
        command: "voltage",
        help: "Display target voltage.",
    },
    HelpTree {
        command: "ws",
        help: "Set/get the wait state on SWD channel. mon ws 5 set the wait states to 5, mon ws gets the current wait states.\n\tThe higher the number the slower it is.",
    },
];
/*
 *
 */
fn _redirect(_command: &str, args: &[&str]) -> bool {
    let onoff: u32 = parsing_util::ascii_string_hex_to_u32(args[0]);
    #[cfg(not(feature = "hosted"))]
    bmp::swindleRedirectLog(onoff != 0);
    encoder::reply_ok();
    true
}
/*
 *
 *
 */
fn _target_reset(_command: &str, _args: &[&str]) -> bool {
    bmp::bmp_platform_nrst_set_val(true);
    // in hosted mode, assume the transport stream will introduce enough delay
    #[cfg(not(feature = "hosted"))]
    rnarduino::rn_os_helper::delay_ms(20);
    bmp::bmp_platform_nrst_set_val(false);
    encoder::reply_ok();
    true
}

/*
 *
 */
fn _reboot(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_e01();
    systemReset();
    true
}
/*
 *
 */
fn _fos_info(_command: &str, _args: &[&str]) -> bool {
    crate::freertos::os_info();
    encoder::reply_ok();
    true
}

/*
 *
 */
fn _fos(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        gdb_print!("Error. Please use :\nmon fos [M0|M3|M4|M33|RV32|NONE|AUTO]\n");
    } else if enable_freertos(args[0]) {
        if crate::freertos::freertos_symbols::freertos_symbol_valid() {
            gdb_print!("FreeRTOS support enabled\n");
        } else {
            gdb_print!("FreeRTOS support *NOT *enabled\n");
        }
    }
    encoder::reply_ok();
    true
}
/*
 *
 */
fn _ram(_command: &str, _args: &[&str]) -> bool {
    let (min_heap, heap) = bmp::get_heap_stats();
    gdb_print!(
        "Min Free Heap\t: {} kB \nFree Heap\t: {} kB\n",
        min_heap >> 10,
        heap >> 10
    );
    encoder::reply_ok();
    true
}

fn _bmp_mon(command: &str, _args: &[&str]) -> bool {
    // the input is bmp actual_bmp_mon_command
    // we have to remove the bmp
    encoder::reply_bool(bmp::bmp_mon(&command[4..]));
    true
}
const MAX_SPACE: usize = 40;
const spacebar: [u8; MAX_SPACE] = [32; MAX_SPACE];
//
fn print_help_tree(helptree: &[HelpTree]) {
    let mut mxsize: usize = 0;
    for i in helptree {
        if i.command.len() > mxsize {
            mxsize = i.command.len();
        }
    }
    if mxsize > MAX_SPACE {
        panic!("padding too big");
    }
    for i in helptree {
        let cmd = i.command;
        let len = cmd.len();
        let pad = core::str::from_utf8(&spacebar[..(mxsize - len)]).unwrap();
        gdb_print!("mon {}{} : {}\n", &cmd, &pad, &(i.help));
    }
}
//
fn _mon_help(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("Help, use mon [cmd] [params] :\n");
    gdb_print!("-----------------------------\n");
    print_help_tree(&help_tree);
    match get_custom_target_help() {
        None => (),
        Some(x) => {
            gdb_print!("Board specific commands :\n");
            gdb_print!("------------------------\n");
            print_help_tree(x);
        }
    }

    encoder::reply_ok();
    true
}

//
fn _boards(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("Support enabled for :\n");
    let boards = bmp::bmp_supported_boards();
    let b: Vec<&str> = boards.split(':').collect();
    for i in b {
        gdb_print!("\t{}\n", i);
    }
    encoder::reply_ok();
    true
}

//
pub fn _voltage(_command: &str, _args: &[&str]) -> bool {
    let voltage = bmp::bmp_get_target_voltage();
    let voltage32: u32 = (voltage * 1000.) as u32;

    gdb_print!("Voltage (mv) : {}\n", voltage32);
    encoder::reply_ok();
    true
}

//
// Execute command
//
pub fn _qRcmd(command: &str, _args: &[&str]) -> bool {
    let largs: Vec<&str> = command.split(',').collect();
    //NOTARGET
    let ln = largs.len();
    if ln != 2 {
        return false;
    }
    // The command is hex encoded, decode it
    let mut out: [u8; 32] = [0; 32];
    let rcmd = match ascii_hex_string_to_u8s(largs[1], &mut out) {
        Ok(x) => x,
        Err(_y) => {
            return false;
        }
    };
    // split command and args
    let command: &[u8];
    let args: &[u8];
    match crate::parsing_util::split_command(rcmd) {
        None => {
            bmpwarning!("Cannot convert string (rcmd)\n");
            return false;
        }
        Some((x, y)) => {
            command = x;
            args = y;
        }
    }
    let as_string = unsafe { core::str::from_utf8_unchecked(command) };
    // Check if it is in the per target tree ...
    match get_custom_target_command() {
        None => (),
        Some(x) => {
            for i in x {
                if as_string.starts_with(i.command) {
                    return exec_one(x, as_string, args);
                }
            }
        }
    }
    // if not, look the generic one
    exec_one(&mon_command_tree, as_string, args)
}

/*
 *
 */
pub fn _get_version(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("{}\n", bmp::bmp_get_version());
    encoder::reply_ok();
    true
}

/*
   Detect stuff connected to the SWD interface
   Try to use the fastest speed
*/
pub fn _swdp_scan(_command: &str, _args: &[&str]) -> bool {
    bmplog!("swdp_scan:\n");

    if !bmp::swdp_scan() {
        bmpwarning!("swdp failed!\n");
        return false;
    }
    crate::freertos::os_detach();
    encoder::reply_ok();
    true
}
/*
   Detect stuff connected to the SWD interface
   Try to use the fastest speed
*/
pub fn _rvswdp_scan(_command: &str, _args: &[&str]) -> bool {
    bmplog!("rvswdp_scan:\n");

    if !bmp::rvswdp_scan() {
        bmpwarning!("rvswdp_scan failed!\n");
        return false;
    }
    crate::freertos::os_detach();
    encoder::reply_ok();
    true
}
/*
 *
 */
pub fn _ws(_command: &str, args: &[&str]) -> bool {
    let len = args.len();
    if len > 0 {
        // ok we have an input
        let ws = ascii_string_decimal_to_u32(args[0]);
        bmp::bmp_set_wait_state(ws);
    }
    let w: u32 = bmp::bmp_get_wait_state();
    gdb_print!("wait states are now {}\n", w);
    encoder::reply_ok();
    true
}
fn convert_param_to_integer(in_str: &str) -> (bool, u32) {
    let trimmed = in_str.trim();
    // ok we have an input
    let mut sz: usize = trimmed.len();
    if sz == 0 {
        gdb_print!("incorrect parameter, expecting xxx or xxxK\n");
        return (false, 0);
    }
    let mut mul: u32 = 1;
    if trimmed.ends_with('k') || trimmed.ends_with('K') {
        mul = 1000;
        sz -= 1;
    }
    let mut out = ascii_string_decimal_to_u32(&trimmed[..sz]);
    out *= mul;
    (true, out)
}
/*
 *
 *
 */
#[unsafe(no_mangle)]
pub fn _fq(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        let f: u32 = bmp::bmp_get_frequency();
        gdb_print!("current frequency is {} \n", f);
        encoder::reply_ok();
        return true;
    }
    let ret: bool;
    let fq: u32;
    (ret, fq) = convert_param_to_integer(args[0]);
    if !ret {
        return false;
    }
    if fq != 0 {
        bmp::bmp_set_frequency(fq);
    } else {
        gdb_print!("incorrect frequency parameter\n");
    }
    let w: u32 = bmp::bmp_get_frequency();
    gdb_print!("frequency is now {} \n", w);
    encoder::reply_ok();
    true
}
#[unsafe(no_mangle)]
static mut autoreset: u32 = 1;
pub fn set_enable_reset(nw: u32) {
    unsafe {
        autoreset = nw;
    }
}
pub fn get_enable_reset() -> u32 {
    unsafe { autoreset }
}
pub fn _enable_reset(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        gdb_print!("current enable_reset is {} \n", get_enable_reset());
        encoder::reply_ok();
        return true;
    }
    let ret: bool;
    let fq: u32;
    (ret, fq) = convert_param_to_integer(args[0]);
    if !ret {
        return false;
    }
    set_enable_reset(fq);
    gdb_print!("enablereset is now {} \n", get_enable_reset());
    encoder::reply_ok();
    true
}
/*
 *
 *
 *
 */
pub fn add_target_commands(target_name: &str) {
    if target_name.starts_with("CH32V2") || target_name.starts_with("CH32V3") {
        set_custom_target_command(
            &crate::commands::mon_ch32vxx::ch32vxx_command_tree,
            &crate::commands::mon_ch32vxx::ch32vxx_help_tree,
        );
    } else {
        clear_custom_target_command();
    }
}

/*
 *
 *
 */
pub fn clear_custom_target_command() {
    unsafe {
        targetCommandTree = None;
        targetHelpTree = None;
    }
}
/*
 *
 *
 */
pub fn set_custom_target_command(
    commandTree: &'static [CommandTree],
    helpTree: &'static [HelpTree],
) {
    unsafe {
        targetCommandTree = Some(commandTree);
        targetHelpTree = Some(helpTree);
    }
}
/*
 *
 *
 */
pub fn get_custom_target_command() -> Option<&'static [CommandTree]> {
    unsafe { targetCommandTree }
}
/*
 *
 *
 */
pub fn get_custom_target_help() -> Option<&'static [HelpTree]> {
    unsafe { targetHelpTree }
}
//-- EOF --
