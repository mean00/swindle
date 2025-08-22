use crate::bmp;
use crate::commands::mon_ch32vxx;
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::freertos;
use crate::freertos::{enable_freertos, freertos_symbols, os_detach};
#[cfg(not(feature = "hosted"))]
use crate::parsing_util;
use crate::parsing_util::{
    ascii_hex_or_dec_to_u32, ascii_string_decimal_to_u32, split_command, u8_hex_string_to_u8s,
};
use crate::setting_keys::*;
use crate::settings;
use alloc::vec::Vec;
//
//
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};
//
pub const MAX_SPACE: usize = 40;
pub const spacebar: [u8; MAX_SPACE] = [32; MAX_SPACE];
//
static mut targetCommandTree: Option<&[CommandTree]> = None;
static mut targetHelpTree: Option<&[HelpTree]> = None;
//
unsafe extern "C" {
    pub fn _Z17lnSoftSystemResetv();
}
unsafe extern "C" {
    pub fn resetTest();
    pub fn resetTest2();
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
const mon_command_tree: [CommandTree; 27] = [
    CommandTree {
        command: "crash",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_crash),
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "map",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_map),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "redirect",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_redirect),
        start_separator: " ",
        next_separator: " ",
    },
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
        command: "delay",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_delay),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "enable_reset_pin",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_enable_reset_pin),
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
        command: "unset",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_unset),
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
        command: "test",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_test),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "test2",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_test2),
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
        command: "rtt",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(crate::commands::mon_rtt::_rtt),
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
        command: "set_reset_pin",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_set_reset_pin),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "set", // that one begins like the one above, it must be after
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_set),
        start_separator: " ",
        next_separator: " ",
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
const help_tree: [HelpTree; 24] = [
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
        command: "delay value",
        help: "wait <value> ms.",
    },
    HelpTree {
        command: "enable_reset_pin value",
        help: "Enable reset through RSTn pin, default is enabled (1).",
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
        command: "map",
        help: "Show the memory map .",
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
        help: "Reset the target by pulsing the reset pin.",
    },
    HelpTree {
        command: "rtt",
        help: "use mon rtt help to get the details.",
    },
    HelpTree {
        command: "rvswdp_scan",
        help: "Probe WCH RISCV device(s).",
    },
    HelpTree {
        command: "set",
        help: "set [key] [value] , just set to get the current set",
    },
    HelpTree {
        command: "unset key",
        help: "unset key",
    },
    HelpTree {
        command: "set_reset_pin value",
        help: "Set the reset pin value to <value>. 1 means pull to ground, 0 mean floating.",
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
#[allow(unused_variables)]
fn _redirect(_command: &str, args: &[&str]) -> bool {
    #[cfg(not(feature = "hosted"))]
    {
        let onoff: u32 = parsing_util::ascii_string_hex_to_u32(args[0]);
        bmp::swindleRedirectLog(onoff != 0);
    }
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
    rust_esprit::delay_ms(20);
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
    freertos::os_info();
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
        if freertos_symbols::freertos_running() {
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
use rust_esprit::rn_gpio::digitalWrite;
//use rust_esprit::rn_gpio::pinMode;
//use rust_esprit::rn_gpio::rnGpioMode::lnOUTPUT;
use rust_esprit::rn_gpio::rnPin;
fn _mon_test(_command: &str, _args: &[&str]) -> bool {
    //unsafe {
    //resetTest();
    //}
    let pin: rnPin = rnPin::PB6;
    let mut state: bool = false;
    gdb_print!("starting test\n");
    //pinMode(pin, lnOUTPUT);
    loop {
        digitalWrite(pin, state);
        state = !state;
        rust_esprit::delay_ms(2000);
    }
    //encoder::reply_ok();
    //true
}
fn _mon_test2(_command: &str, _args: &[&str]) -> bool {
    unsafe {
        resetTest2();
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
const MAX_RCMD_SIZE: usize = 128;
pub fn _qRcmd(_command: &str, args: &[&str]) -> bool {
    if args.len() != 1 {
        gdb_print!("qRCmd : wrong args\n");
        return false;
    }
    if args[0].len() > 2 * MAX_RCMD_SIZE {
        //
        gdb_print!("qRCmd : too long {}\n", args[0].len());
        return false;
    }
    // The command is hex encoded, decode it
    let mut out: [u8; MAX_RCMD_SIZE] = [0; MAX_RCMD_SIZE];
    let rcmd = u8_hex_string_to_u8s(args[0].as_bytes(), &mut out);
    // split command and args
    let command: &[u8];
    let args: &[u8];
    match split_command(rcmd) {
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
    os_detach();
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
    os_detach();
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
pub fn _enable_reset_pin(_command: &str, args: &[&str]) -> bool {
    let ret: bool;
    let fq: u32;
    (ret, fq) = convert_param_to_integer(args[0]);
    if !ret {
        return false;
    }
    set_enable_reset(fq);
    gdb_print!("enable reset pin is now {} \n", get_enable_reset());
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
            &mon_ch32vxx::ch32vxx_command_tree,
            &mon_ch32vxx::ch32vxx_help_tree,
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
fn _map(_command: &str, _args: &[&str]) -> bool {
    let ram: Vec<bmp::MemoryBlock> = bmp::bmp_get_mapping(bmp::mapping::Ram);
    for i in ram {
        gdb_print!(
            " RAM   : start=0x{:x} size={} kB\n",
            i.start_address,
            i.length / 1024
        );
    }
    let flash: Vec<bmp::MemoryBlock> = bmp::bmp_get_mapping(bmp::mapping::Flash);
    for i in flash {
        gdb_print!(
            " FLASH : start=0x{:x} size={} kB\n",
            i.start_address,
            i.length / 1024
        );
    }
    encoder::reply_ok();
    true
}
/*
 *
 *
 */
fn _crash(_command: &str, _args: &[&str]) -> bool {
    bmp::bmp_raise_exception();
    false
}

/*
 *
 *
 *
 */
fn _set(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        settings::dump();
        encoder::reply_ok();
        return true;
    }
    if args.len() == 2 {
        let value: u32 = ascii_hex_or_dec_to_u32(args[1]);
        settings::set(args[0], value);
        encoder::reply_ok();
        return true;
    }
    false
}
/*
 *
 *
 */
fn _unset(_command: &str, args: &[&str]) -> bool {
    match args.len() {
        0 => settings::dump(),
        1 => {
            if !settings::remove(args[0]) {
                gdb_print!("That key does not exist\n");
                return false;
            }
        }
        _ => return false,
    }
    encoder::reply_ok();
    true
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
/*
*
*
*/
pub fn _delay(_command: &str, args: &[&str]) -> bool {
    let ret: bool;
    let val: u32;
    (ret, val) = convert_param_to_integer(args[0]);
    if !ret {
        return false;
    }
    #[cfg(not(feature = "hosted"))]
    rust_esprit::delay_ms(val);
    encoder::reply_ok();
    true
}
/*
*
*
*/
#[unsafe(no_mangle)]
pub fn _set_reset_pin(_command: &str, args: &[&str]) -> bool {
    let ret: bool;
    let val: u32;
    (ret, val) = convert_param_to_integer(args[0]);
    if !ret {
        gdb_print!("set_pin_reset bad parameter  \n");
        return false;
    }
    bmp::bmp_platform_nrst_set_val(val != 0);
    gdb_print!("reset pin is now {} \n", (val != 0));
    encoder::reply_ok();
    true
}
unsafe extern "C" {
    pub fn platform_nrst_set_val_internal(set: u32);
}
fn set_nrst(set: bool) {
    let d: u32 = match set {
        false => 0,
        _ => 1,
    };
    unsafe { platform_nrst_set_val_internal(d) };
}
#[unsafe(no_mangle)]
pub fn platform_nrst_set_val(set: u32) {
    let delay: u32 = match set {
        0 => {
            // clear
            let d = settings::get_or_default(RESET_HOLDOFF_DURATION, 10);
            gdb_print!("reset off, holdoff  {} \n", d);
            set_nrst(false);
            d
        }
        _ => {
            // set
            let d = settings::get_or_default(RESET_PULSE_DURATION, 10);
            gdb_print!("reset on, duration  {} \n", d);
            set_nrst(true);
            d
        }
    };
    rust_esprit::delay_ms(delay);
}

//-- EOF --
