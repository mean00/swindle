use crate::bmp;
use crate::commands::{exec_one, CallbackType, CommandTree};
use crate::encoder::encoder;
use crate::freertos::enable_freertos;
use crate::parsing_util::{
    ascii_hex_string_to_u8s, ascii_string_decimal_to_u32, ascii_string_to_u32,
};
use alloc::vec;
use alloc::vec::Vec;
//
//
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};
extern "C" {
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
struct HelpTree {
    command: &'static str,
    help: &'static str,
}

//
const mon_command_tree: [CommandTree; 16] = [
    CommandTree {
        command: "help",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_help),
    }, //
    CommandTree {
        command: "swdp_scan",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_swdp_scan),
    }, //
    CommandTree {
        command: "rvswdp_scan",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_rvswdp_scan),
    }, //
    CommandTree {
        command: "voltage",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_voltage),
    }, //
    CommandTree {
        command: "boards",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_boards),
    }, //
    CommandTree {
        command: "version",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_get_version),
    }, //
    CommandTree {
        command: "bmp",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_bmp_mon),
    }, //
    CommandTree {
        command: "ram",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_ram),
    }, //
    CommandTree {
        command: "ws",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_ws),
    }, //
    CommandTree {
        command: "frequency",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
    },
    CommandTree {
        command: "fq",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
    },
    CommandTree {
        command: "fos",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
    }, //
    CommandTree {
        command: "os_info",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos_info),
    }, //
    CommandTree {
        command: "freertos",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
    }, //
    CommandTree {
        command: "reboot",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_reboot),
    }, //
    CommandTree {
        command: "reset",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_target_reset),
    }, //
];
//
const help_tree : [HelpTree;14]=
[
    HelpTree{ command: "help",help :"Display help." },
    HelpTree{ command: "bmp",help :"Forward the command to bmp mon command.\n\tExample : mon bmp mass_erase is the same as mon mass_erase on a bmp.." },
    HelpTree{ command: "boards",help :"Display supported boards. This is set at build time." },
    HelpTree{ command: "fq or frequency",help :"set/get SWD frequency."},
    HelpTree{ command: "fos",help :"Enable FreeRTOS support." },    
    HelpTree{ command: "os_info",help :"Dump FreeRTOS internal state." },
    HelpTree{ command: "ram",help :"Display stats about Ram usage." },
    HelpTree{ command: "reboot",help :"Reboot the debugger." },
    HelpTree{ command: "reset",help :"Reset the target." },
    HelpTree{ command: "rvswdp_scan",help:"Probe WCH RISCV device(s)." },
    HelpTree{ command: "swdp_scan",help :"Probe device(s) over SWD. You might want to increase wait state if it fails." },
    HelpTree{ command: "version",help :"Display version." },
    HelpTree{ command: "voltage",help :"Display target voltage." },
    HelpTree{ command: "ws",help :"Set/get the wait state on SWD channel. mon ws 5 set the wait states to 5, mon ws gets the current wait states.\n\tThe higher the number the slower it is." },
];
/*
 *
 */
pub fn _target_reset(_command: &str, _args: &[&str]) -> bool {
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
pub fn _reboot(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_e01();
    systemReset();
    true
}
/*
 *
 */
pub fn _fos_info(_command: &str, _args: &[&str]) -> bool {
    crate::freertos::os_info();
    encoder::reply_ok();
    true
}

/*
 *
 */
pub fn _fos(command: &str, _args: &[&str]) -> bool {
    let mut flavor: &str = "";
    if command.len() > 4 {
        flavor = &command[4..];
    }

    if enable_freertos(flavor) {
        if crate::freertos::freertos_symbols::freertos_symbol_valid() {
            gdb_print!("FreeRTOS support enabled\n");
        } else {
            gdb_print!("FreeRTOS support *NOT *enabled\n");
        }
        encoder::reply_ok();
        true
    } else {
        gdb_print!("Error. Please use :\nmon fos [M0|M3|M4|M33|RV32|NONE|AUTO]\n");
        encoder::reply_e01();
        true
    }
}
/*
 *
 */
pub fn _ram(_command: &str, _args: &[&str]) -> bool {
    let (min_heap, heap) = bmp::get_heap_stats();
    gdb_print!(
        "Min Free Heap\t: {} kB \nFree Heap\t: {} kB\n",
        min_heap >> 10,
        heap >> 10
    );
    encoder::reply_ok();
    true
}

pub fn _bmp_mon(command: &str, _args: &[&str]) -> bool {
    // the input is bmp actual_bmp_mon_command
    // we have to remove the bmp
    encoder::reply_bool(bmp::bmp_mon(&command[4..]));
    true
}

//
pub fn _mon_help(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("Help, use mon [cmd] [params] :\n");
    for i in help_tree {
        gdb_print!("mon {} \t: {}\n", &(i.command), &(i.help));
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
pub fn _ws(_command: &str, _args: &[&str]) -> bool {
    let len = _command.len();
    if len > 3 {
        // ok we have an input
        let ws = ascii_string_decimal_to_u32(&_command[3..]);
        bmp::bmp_set_wait_state(ws);
    }
    let w: u32 = bmp::bmp_get_wait_state();
    gdb_print!("wait states are now {}\n", w);
    encoder::reply_ok();
    true
}
/*
 *
 *
 */
#[no_mangle]
pub fn _fq(command: &str, _args: &[&str]) -> bool {
    let params: Vec<&str> = command.split(' ').collect();
    if params.len() <= 1 {
        let f: u32 = bmp::bmp_get_frequency();
        gdb_print!("current frequency is {} \n", f);
        encoder::reply_ok();
        return true;
    }
    let fq_str = params[1].trim();
    // ok we have an input
    let mut sz: usize = fq_str.len();
    if sz == 0 {
        gdb_print!("incorrect parameter, expecting xxx or xxxK\n");
        return false;
    }
    let mut mul: u32 = 1;
    if fq_str.ends_with('k') || fq_str.ends_with('K') {
        mul = 1000;
        sz = sz - 1;
    }
    let mut frequency = ascii_string_decimal_to_u32(&fq_str[..sz]);
    if frequency != 0 {
        frequency = frequency * mul;
        bmp::bmp_set_frequency(frequency);
    } else {
        gdb_print!("incorrect frequency parameter\n");
    }
    let w: u32 = bmp::bmp_get_frequency();
    gdb_print!("frequency is now {} \n", w);
    encoder::reply_ok();
    true
}
//-- EOF --
