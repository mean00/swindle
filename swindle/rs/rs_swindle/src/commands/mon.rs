//! GDB `monitor` command handler — debugger control and diagnostics.
//!
//! Implements the GDB `monitor` (aka `qRcmd`) command, providing a rich
//! set of sub-commands for controlling the debugger probe and inspecting
//! target state:
//!
//! | Command | Description |
//! |---------|-------------|
//! | `swdp_scan` | Probe SWD bus for ARM devices |
//! | `rvswdp_scan` | Probe for RISC-V devices |
//! | `sdi_scan` | Probe WCH CH32V0xx single-wire (SDI) devices |
//! | `fq` / `frequency` | Get/set SWD clock frequency |
//! | `ws` | Get/set SWD wait states |
//! | `voltage` | Read target supply voltage |
//! | `reset` | Pulse the target reset pin |
//! | `reboot` | Reboot the debugger probe |
//! | `version` | Show firmware version |
//! | `map` | Show target memory map |
//! | `xml reg` / `xml memory` | Dump target.xml / memory-map XML exactly as GDB receives it |
//! | `ram` | Show debugger heap usage |
//! | `stacks` | Per-task stack high-water marks of the probe's own FreeRTOS kernel |
//! | `boards` | List supported target boards |
//! | `set` / `unset` | Get/set/remove persistent settings |
//! | `rtt` | RTT control (see `mon_rtt.rs`) |
//! | `freertos` / `fos` | Enable FreeRTOS task awareness |
//! | `os_info` | Dump FreeRTOS internal state |
//! | `redirect` | Redirect logging to USB CDC |
//! | `bmp` | Forward command to BMP monitor |
//! | `riscv_benchmark` | Run the RISC-V memory-access benchmark (abstract, progbuf, sysbus) |
//! | `riscv_memory_benchmark` | Re-run the benchmark through the generic target memory API only |
//! | `riscv_stream` | Select the program-buffer streaming sub-path (BMP 103 A/B knob) |
//! | `riscv_confirm` | Show/clear the confirmed-read counters: the silent read losses (BMP 103) |
//! | `help` | Show this help |
//! | `crash` | Force a crash (for testing) |
//! | `delay` | Wait N milliseconds |
//! | `breakpoint_info` | Show HW breakpoint/watchpoint count |
//! | `enable_reset_pin` | Enable/disable reset pin control |
//! | `set_reset_pin` | Manually set reset pin state |
//! | `help` | Show this help, plus the attached driver's own commands |

use core::sync::atomic::{AtomicU32, Ordering};

use crate::bmp;
use crate::commands::{CallbackType, CommandTree, exec_one};
use crate::encoder::encoder;
use crate::freertos;
use crate::freertos::{enable_freertos, freertos_symbols, os_detach};
#[cfg(not(feature = "hosted"))]
use crate::parsing_util;
use crate::parsing_util::{
    ascii_hex_or_dec_to_u32, ascii_string_decimal_to_u32, convert_param_to_integer, string_to_bool,
    u8_hex_string_to_u8s,
};
use crate::riscv::{
    RV_CSR_DCSR, RV_CSR_DPC, RV_CSR_DSCRATCH0, RV_CSR_DSCRATCH1, RV_CSR_MCAUSE, RV_CSR_MEPC,
    RV_CSR_MIE, RV_CSR_MIP, RV_CSR_MISA, RV_CSR_MSCRATCH, RV_CSR_MSTATUS, RV_CSR_MTVAL,
    RV_CSR_MTVEC, RV_CSR_TDATA1, RV_CSR_TDATA2, RV_CSR_TDATA3, RV_CSR_TINFO, RV_CSR_TSELECT,
};
use crate::setting_keys::*;
use crate::settings;
//#[cfg(feature = "hosted")]

//#[cfg(not(feature = "hosted"))]
//use rust_esprit::rn_gpio::rnPin;

//
#[cfg(not(feature = "hosted"))]
use rust_esprit::delay_ms;
//#[cfg(not(feature = "hosted"))]
//use rust_esprit::rn_gpio::digital_write;
#[cfg(feature = "hosted")]
fn delay_ms(_a: u32) {}

//
setup_log!(false);
crate::gdb_print_init!();
//use crate::{bmplog, bmpwarning, gdb_print};
//
pub const MAX_SPACE: usize = 40;
pub const spacebar: [u8; MAX_SPACE] = [32; MAX_SPACE];
//
use crate::SyncCell;

static targetCommandTree: SyncCell<Option<&'static [CommandTree]>> = SyncCell::new(None);

/// A set of `mon` commands declared by a Rust target driver:
/// `(driver name, commands)`.
type DriverCommandSet = (&'static str, &'static [CommandTree]);

/// How many target drivers may declare their own `mon` commands.
const MAX_DRIVER_COMMAND_SETS: usize = 16;

/// Commands declared by the probed drivers, keyed by driver name.
///
/// A driver declares them through `RustRiscvTargetDriver::commands()` /
/// `RustArmTargetDriver::commands()` and its shim registers them here at probe time;
/// `add_target_commands()` then installs the set that matches the attached target's
/// driver name.
static driverCommandSets: SyncCell<[Option<DriverCommandSet>; MAX_DRIVER_COMMAND_SETS]> =
    SyncCell::new([None; MAX_DRIVER_COMMAND_SETS]);
//
unsafe extern "C" {
    pub fn _Z17lnSoftSystemResetv();
}

/// The probe's own FreeRTOS task table, for `mon stacks`.
///
/// `uxTaskGetSystemState()` is compiled in because `configUSE_TRACE_FACILITY` is
/// 1 (`esprit/freertos_config/FreeRTOSConfig.h`), and that is also what fills
/// `TaskStatus_t.usStackHighWaterMark`: `prvTaskCheckFreeStackSpace()`
/// (`FreeRTOS/tasks.c:6315`) scans the `tskSTACK_FILL_BYTE` pattern from the low
/// end of the stack and reports the **free** part in `StackType_t` words, so the
/// peak use is `stack size - STACK_WORD_BYTES * stack_high_water_mark`.
/// `INCLUDE_uxTaskGetStackHighWaterMark` is 0, so this is the only way to read
/// that number - the standalone `uxTaskGetStackHighWaterMark()` is not linked.
///
/// `TaskStatus` mirrors FreeRTOS `TaskStatus_t` (`tasks.h`): the same layout the
/// target-side binding in `esprit/rust/rs_rtos/src/c_freertos.rs` uses.
#[cfg(not(feature = "hosted"))]
mod probe_kernel {
    use core::ffi::{c_char, c_void};

    /// Probe tasks (idle, timer, usbd, gdb, usbserial, ...) - 8 leaves headroom.
    pub const MAX_TASKS: usize = 8;

    /// `sizeof(StackType_t)`, i.e. `FREERTOS_SIZEOF_StackType_t` in the
    /// generated `freertos_struct.h` (32-bit cores here).
    pub const STACK_WORD_BYTES: u32 = 4;

    /// FreeRTOS `eTaskState` (`tasks.h`).
    pub const STATE_RUNNING: u32 = 0;
    pub const STATE_READY: u32 = 1;
    pub const STATE_BLOCKED: u32 = 2;
    pub const STATE_SUSPENDED: u32 = 3;
    pub const STATE_DELETED: u32 = 4;

    /// FreeRTOS `TaskStatus_t` (`tasks.h`).
    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct TaskStatus {
        pub handle: *mut c_void,
        pub name: *const c_char,
        pub task_number: u32,
        pub state: u32,
        pub current_priority: u32,
        pub base_priority: u32,
        pub run_time_counter: u32,
        pub stack_base: *mut c_void,
        /// Free part of the stack, in `StackType_t` words.
        pub stack_high_water_mark: u32,
    }

    impl TaskStatus {
        pub const fn empty() -> Self {
            Self {
                handle: core::ptr::null_mut(),
                name: core::ptr::null(),
                task_number: 0,
                state: STATE_DELETED,
                current_priority: 0,
                base_priority: 0,
                run_time_counter: 0,
                stack_base: core::ptr::null_mut(),
                stack_high_water_mark: 0,
            }
        }
    }

    /// One-letter form of `eTaskState`.
    pub fn state_letter(state: u32) -> &'static str {
        match state {
            STATE_RUNNING => "R",
            STATE_READY => "r",
            STATE_BLOCKED => "B",
            STATE_SUSPENDED => "S",
            STATE_DELETED => "D",
            _ => "?",
        }
    }

    unsafe extern "C" {
        /// Snapshot up to `array_size` tasks; returns how many were written.
        pub fn uxTaskGetSystemState(
            status_array: *mut TaskStatus,
            array_size: u32,
            total_run_time: *mut u32,
        ) -> u32;
        /// Number of tasks (idle included) at this instant.
        pub fn uxTaskGetNumberOfTasks() -> u32;
    }
}

/*
 *
 */
/// Perform a soft system reset of the debugger probe.
fn systemReset() {
    unsafe {
        _Z17lnSoftSystemResetv();
    }
}

//
const mon_command_tree: [CommandTree; 44] = [
    CommandTree {
        command: "help",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_help),
        start_separator: b' ',
        next_separator: b' ',
        help: "Display help.",
    },
    CommandTree {
        command: "bmp",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_bmp_mon),
        start_separator: 0,
        next_separator: 0,
        help: "Forward the command to bmp mon command.\n\tExample : mon bmp mass_erase is the same as mon mass_erase on a bmp..",
    },
    CommandTree {
        command: "breakpoint_info",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_breakpoint_count),
        start_separator: 0,
        next_separator: 0,
        help: "Display the total number of hardware breakpoints and watchpoints.",
    },
    CommandTree {
        command: "boards",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_boards),
        start_separator: 0,
        next_separator: 0,
        help: "Display supported boards. This is set at build time.",
    },
    CommandTree {
        command: "csr",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(_csr),
        start_separator: b' ',
        next_separator: b' ',
        help: "Read and dump RISC-V CSR (e.g. mon csr dcsr, mon csr 0x7b0). Only for RISC-V targets.",
    },
    CommandTree {
        command: "delay",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_delay),
        start_separator: 0,
        next_separator: 0,
        help: "wait <value> ms.",
    },
    CommandTree {
        command: "enable_reset_pin",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_enable_reset_pin),
        start_separator: 0,
        next_separator: 0,
        help: "Enable reset through RSTn pin, default is enabled (1).",
    },
    CommandTree {
        command: "erase",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_erase),
        start_separator: b' ',
        next_separator: b' ',
        help: "Erase the target flash pages covering <start>..<start>+<length>; both take decimal or 0x hex.",
    },
    CommandTree {
        command: "frequency",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
        start_separator: b' ',
        next_separator: b' ',
        help: "set/get SWD frequency.",
    },
    CommandTree {
        command: "fq",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_fq),
        start_separator: b' ',
        next_separator: b' ',
        help: "",
    },
    CommandTree {
        command: "memlog",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_memlog),
        start_separator: b' ',
        next_separator: b' ',
        help: "Phase 0 instrumentation: enable/disable per-call [MEMLOG] lines; reset SWD/RVSWD tx counters.",
    },
    CommandTree {
        command: "fos",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
        start_separator: b' ',
        next_separator: b' ',
        help: "Enable FreeRTOS support.",
    },
    CommandTree {
        command: "freertos",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos),
        start_separator: b' ',
        next_separator: b' ',
        help: "",
    },
    CommandTree {
        command: "map",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_map),
        start_separator: b' ',
        next_separator: b' ',
        help: "Show the memory map .",
    },
    CommandTree {
        command: "os_info",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_fos_info),
        start_separator: b' ',
        next_separator: b' ',
        help: "Dump FreeRTOS internal state.",
    },
    CommandTree {
        command: "ram",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_ram),
        start_separator: 0,
        next_separator: 0,
        help: "Display stats about Ram usage.",
    },
    CommandTree {
        command: "arena",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_arena),
        start_separator: 0,
        next_separator: 0,
        help: "Session arena usage: transport/pool bump regions, frees, rewinds.",
    },
    CommandTree {
        command: "stacks",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_stacks),
        start_separator: 0,
        next_separator: 0,
        help: "Per-task stack high-water marks (free bytes left at the worst moment).",
    },
    CommandTree {
        command: "reboot",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_reboot),
        start_separator: 0,
        next_separator: 0,
        help: "Reboot the debugger.",
    },
    CommandTree {
        command: "riscv_benchmark",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_riscv_benchmark),
        start_separator: 0,
        next_separator: 0,
        help: "Benchmark the RISC-V memory access methods (abstract, progbuf, sysbus).",
    },
    CommandTree {
        command: "riscv_memory_benchmark",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_riscv_memory_benchmark),
        start_separator: 0,
        next_separator: 0,
        help: "Benchmark the RISC-V memory access via the generic target API (verifies best-path routing).",
    },
    CommandTree {
        command: "redirect",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_redirect),
        start_separator: b' ',
        next_separator: b' ',
        help: "Redirect log to usb2. mon redirect 1 to redirect, mon redirect 0 to not disable",
    },
    CommandTree {
        command: "reset",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_target_reset),
        start_separator: 0,
        next_separator: 0,
        help: "Reset the target by pulsing the reset pin.",
    },
    CommandTree {
        command: "rtt",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(crate::commands::mon_rtt::_rtt),
        start_separator: 0,
        next_separator: 0,
        help: "use mon rtt help to get the details.",
    },
    CommandTree {
        command: "rvswdp_scan",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_rvswdp_scan),
        start_separator: 0,
        next_separator: 0,
        help: "Probe WCH RISCV device(s).",
    },
    CommandTree {
        command: "sdi_scan",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(crate::riscv_extra::_sdi_scan),
        start_separator: 0,
        next_separator: 0,
        help: "Probe WCH CH32V0xx device(s) over the single-wire SDI interface.",
    },
    CommandTree {
        command: "stub_stats",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_stub_stats),
        start_separator: b' ',
        next_separator: b' ',
        help: "Show (or 'mon stub_stats reset') the flash-stub statistics: runs, halts seen, register-repair fallbacks, poll wait/hold and the last failing run's state.",
    },
    CommandTree {
        command: "stub_hold",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_stub_hold),
        start_separator: b' ',
        next_separator: b' ',
        help: "Show/set the per-page flash-stub hold in ms: 'mon stub_hold <ms>' ('0' = poll from the start) or 'mon stub_hold auto' for the part's own value.",
    },
    CommandTree {
        command: "sdi_wire",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(crate::riscv_extra::_sdi_wire),
        start_separator: b' ',
        next_separator: b' ',
        help: "Override the SDI bit-bang wire timing in ns (0 = keep the tap's default); 'mon sdi_scan' re-calibrates with it.",
    },
    CommandTree {
        command: "riscv_stream",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_riscv_stream),
        start_separator: b' ',
        next_separator: b' ',
        help: "Select the RISC-V program-buffer streaming sub-path (BMP 103): 'off' makes every word wait for its own command, which is the A/B for the silent read losses (riscv_fault.md 12.9).",
    },
    CommandTree {
        command: "riscv_confirm",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_riscv_confirm),
        start_separator: b' ',
        next_separator: b' ',
        help: "Show (or clear) the RISC-V confirmed-read counters (BMP 103): windows, disagreements, faults, latched command errors, abstractauto retries, and where the last disagreement was.",
    },
    CommandTree {
        command: "set_reset_pin",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_set_reset_pin),
        start_separator: 0,
        next_separator: 0,
        help: "Set the reset pin value to <value>. 1 means pull to ground, 0 mean floating.",
    },
    CommandTree {
        command: "set",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_set),
        start_separator: b' ',
        next_separator: b' ',
        help: "set [key] [value] , just set to get the current set",
    },
    CommandTree {
        command: "unset",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_unset),
        start_separator: b' ',
        next_separator: b' ',
        help: "unset key",
    },
    CommandTree {
        command: "swdp_scan",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_swdp_scan),
        start_separator: 0,
        next_separator: 0,
        help: "Probe device(s) over SWD. You might want to increase wait state if it fails.",
    },
    CommandTree {
        command: "targets",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_targets),
        start_separator: 0,
        next_separator: 0,
        help: "Display discovered targets.",
    },
    CommandTree {
        command: "vector_catch",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_vector_catch),
        start_separator: b' ',
        next_separator: b' ',
        help: "Catch exception vectors (enable/disable)",
    },
    CommandTree {
        command: "version",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_get_version),
        start_separator: 0,
        next_separator: 0,
        help: "Display version.",
    },
    CommandTree {
        command: "voltage",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_voltage),
        start_separator: 0,
        next_separator: 0,
        help: "Display target voltage.",
    },
    CommandTree {
        command: "ws",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_ws),
        start_separator: b' ',
        next_separator: b' ',
        help: "Set/get the wait state on SWD channel. mon ws 5 set the wait states to 5, mon ws gets the current wait states.\n\tThe higher the number the slower it is.\nYou should rather use mon frequency.",
    },
    CommandTree {
        command: "xml",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(_xml),
        start_separator: b' ',
        next_separator: b' ',
        help: "Dump the GDB XML description: mon xml reg (target.xml / registers), mon xml memory (memory map).",
    },
    CommandTree {
        command: "crash",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_crash),
        start_separator: 0,
        next_separator: 0,
        help: "",
    },
    CommandTree {
        command: "test",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_test),
        start_separator: b' ',
        next_separator: b' ',
        help: "",
    },
    CommandTree {
        command: "test2",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mon_test2),
        start_separator: b' ',
        next_separator: b' ',
        help: "",
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
    // Target is being reset: cached memory lines are stale.
    crate::mem_cache::invalidate();
    swindle_nrst_set_val(1);
    swindle_nrst_set_val(0);
    encoder::reply_ok();
    true
}

/*
 *
 */
/// Print the `mon erase` usage lines.
fn _erase_usage() {
    gdb_print!("usage: mon erase <start> <length>\n");
    gdb_print!("\t<start> is an address in the first page to erase\n");
    gdb_print!("\t<length> is the number of bytes after it, rounded up to whole pages\n");
    gdb_print!("\tboth take decimal (1024) or 0x hexadecimal (0x400)\n");
}

/// Handle `mon erase <start> <length>` — erase the flash pages covering the range.
///
/// `<start>` is an address inside the first page to erase and `<length>` the number of
/// bytes after it (rounded up to whole pages); both take decimal or `0x` hexadecimal.
/// This is the same `target_flash_erase()` walk GDB's `vFlashErase` does, so the flash
/// banks of the attached driver get erased with their own geometry.
fn _erase(_command: &str, args: &[&str]) -> bool {
    if args.len() < 2 {
        _erase_usage();
        encoder::reply_e01();
        return true;
    }
    let (start, length) = match (
        crate::parsing_util::try_ascii_hex_or_dec_to_u32(args[0]),
        crate::parsing_util::try_ascii_hex_or_dec_to_u32(args[1]),
    ) {
        (Some(start), Some(length)) => (start, length),
        _ => {
            gdb_print!("mon erase : <start> and <length> must be decimal or 0x hex numbers\n");
            _erase_usage();
            encoder::reply_e01();
            return true;
        }
    };
    if length == 0 {
        gdb_print!("mon erase : length must not be zero\n");
        encoder::reply_e01();
        return true;
    }
    if !bmp::bmp_flash_erase(start, length) {
        // `target_flash_erase()` fails when the range is not inside the target flash.
        gdb_print!("mon erase : no flash at ", Hex(start), "\n");
        encoder::reply_e01();
        return true;
    }
    gdb_print!("erased ", Hex(length), " bytes at ", Hex(start), "\n");
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
        "Min Free Heap\t: ",
        min_heap >> 10,
        " kB \nFree Heap\t: ",
        heap >> 10
    );
    gdb_print!("kB\n");
    encoder::reply_ok();
    true
}

/// Handle `mon arena` — the session-scoped static arenas (see
/// `rs_swindle/src/session_arena.rs`, malloc_used.md).
///
/// `tr` is the transport region - today only the `riscv_dmi_s` the RVSWD/SDI
/// templates allocate, which is why it reads 40 B - and `pool` the per-target
/// objects (the DP/AP chain included).  Both are strict bump regions with a
/// single lifetime: they are wiped when the target list is freed, i.e. at the
/// start of every scan.  `freed` is the number of `free()` calls that landed on
/// arena memory, `rewound` how many of those gave their bytes back (the block
/// was the top one - a bump region can only reclaim in LIFO order).  An
/// allocation that does not fit is fatal, so `used` never exceeds the size and
/// the peaks are the numbers to check against it: read them after the workload
/// of interest (`sdi_scan`/`rvswdp_scan`/`swdp_scan`, `attach 1`, `load`,
/// `compare-sections`).
fn _arena(_command: &str, _args: &[&str]) -> bool {
    let a = crate::session_arena::stats();
    if !crate::session_arena::BUILT_IN {
        gdb_println!("Session arena : not built in (SWINDLE_SESSION_ARENA=OFF)");
        encoder::reply_ok();
        return true;
    }
    gdb_println!(
        "Session arena : armed ",
        a.armed,
        " freed ",
        a.frees,
        " rewound ",
        a.rewinds
    );
    gdb_println!(
        "  tr   : ",
        a.transport_used,
        " / ",
        a.transport_size,
        " B (peak ",
        a.transport_peak,
        ")"
    );
    gdb_println!(
        "  pool : ",
        a.pool_used,
        " / ",
        a.pool_size,
        " B (peak ",
        a.pool_peak,
        ")"
    );
    encoder::reply_ok();
    true
}

/// Handle `mon stacks` — per-task stack high-water marks of the probe's own
/// FreeRTOS kernel, i.e. the numbers the task stack sizes are right-sized from.
///
/// `free` is what is left of the stack at its worst moment so far (words and
/// bytes), so the peak use is `TASK_*_STACK_SIZE - free`.  The mark only ever
/// moves down, so run the workload of interest first: attach a target, `load` a
/// 200 KB image, `compare-sections`, `mon riscv_memory_benchmark`, then read it.
#[cfg(not(feature = "hosted"))]
fn _stacks(_command: &str, _args: &[&str]) -> bool {
    use core::str;
    use probe_kernel::*;

    /// Room for the longest task name (`configMAX_TASK_NAME_LEN` - 1).
    const NAME_WIDTH: usize = 16;

    let mut status = [TaskStatus::empty(); MAX_TASKS];
    let mut total_run_time: u32 = 0;
    let got =
        unsafe { uxTaskGetSystemState(status.as_mut_ptr(), MAX_TASKS as u32, &mut total_run_time) };

    gdb_println!("task           state prio  free stack (worst case so far)");
    for t in &status[..got as usize] {
        let name = if t.name.is_null() {
            b"?".as_slice()
        } else {
            unsafe { core::ffi::CStr::from_ptr(t.name) }.to_bytes()
        };
        let pad = NAME_WIDTH.saturating_sub(name.len());
        gdb_print!(
            str::from_utf8(name).unwrap_or("?"),
            str::from_utf8(&spacebar[..pad]).unwrap(),
            state_letter(t.state),
            "     ",
            t.current_priority,
            "    ",
            t.stack_high_water_mark,
            " w / ",
            t.stack_high_water_mark * STACK_WORD_BYTES,
            " B\n"
        );
    }
    let nb = unsafe { uxTaskGetNumberOfTasks() };
    if nb as usize > MAX_TASKS {
        gdb_print!(
            "only the first ",
            MAX_TASKS,
            " of ",
            nb,
            " tasks are shown\n"
        );
    }
    gdb_print!("total run time : ", total_run_time);
    gdb_print!(" ticks\n");
    encoder::reply_ok();
    true
}

/// Handle `mon stacks` on the host: the probe's kernel is not in this process.
#[cfg(feature = "hosted")]
fn _stacks(_command: &str, _args: &[&str]) -> bool {
    gdb_println!("mon stacks : not available in hosted mode (no probe kernel here)");
    encoder::reply_ok();
    true
}

fn _bmp_mon(command: &str, _args: &[&str]) -> bool {
    // the input is bmp actual_bmp_mon_command
    // we have to remove the bmp
    encoder::reply_bool(bmp::bmp_mon(&command[4..]));
    true
}
/*
 *
 *
 */
/// Handle `mon riscv_benchmark` — run the RISC-V memory-access benchmark.
///
/// The benchmark is implemented on the BMP side as a RISC-V target command
/// (`riscv32_cmd_list` entry `riscv_benchmark`). Forward it straight through —
/// same as `mon bmp riscv_benchmark`. Requires a connected RISC-V (CH32)
/// target; BMP reports an error otherwise.
fn _riscv_benchmark(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_bool(bmp::bmp_run_riscv_benchmark());
    true
}
/// Handle `mon riscv_benchmark2` — run the RISC-V memory benchmark through
/// the generic target memory API (`target_mem64_write/read`) only, to confirm
/// the auto-selected "best" method is the one the normal path actually uses.
/// Same forwarding as `mon riscv_benchmark`; prints exactly one result line.
fn _riscv_memory_benchmark(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_bool(bmp::bmp_run_riscv_benchmark2());
    true
}
/// Handle `mon riscv_stream [on|off]` — BMP 103: select the program-buffer streaming
/// sub-path of the RISC-V memory reader and writer.
///
/// One read, one wire, and only this implementation detail differing: with the stream on
/// (the default) DATA0 is read without ever waiting for the command that read starts; with
/// it off every word goes through its own command plus completion wait. That is the A/B
/// `riscv_fault.md` §12.9 is measured with.
fn _riscv_stream(_command: &str, args: &[&str]) -> bool {
    if let Some(arg) = args.first() {
        match *arg {
            "on" => bmp::bmp_set_riscv_progbuf_stream(true),
            "off" => bmp::bmp_set_riscv_progbuf_stream(false),
            _ => gdb_println!("usage: mon riscv_stream on|off"),
        }
    }
    let state = if bmp::bmp_get_riscv_progbuf_stream() {
        "on"
    } else {
        "off"
    };
    gdb_println!(
        "riscv stream : ",
        state,
        " (program buffer streaming sub-path)"
    );
    encoder::reply_ok();
    true
}
/// Handle `mon riscv_confirm [reset]` — BMP 103: the counters behind the confirmed read.
///
/// On a transport that cannot report a lost access (the SDI wire, `dmi->read_confirm`) every
/// read window is read twice and the two passes compared, because the debug module answers a
/// read it dropped with the *previous* answer while busy and cmderr stay clear. These are the
/// silent losses nothing else reports:
/// `windows` windows confirmed, `disagreeing` windows whose passes disagreed, `elements`
/// elements settled one at a time afterwards, `faults` passes that reported a fault (patch
/// 102's re-issue), `latched` transfers that ended with a command error latched, `autoexec`
/// abstractauto writes that had to be repeated, and the last disagreement (`last_address`,
/// `last_first_pass`, `last_second_pass`). `reset` zeroes them.
fn _riscv_confirm(_command: &str, args: &[&str]) -> bool {
    if !args.is_empty() {
        match args[0] {
            "reset" => {
                bmp::bmp_riscv_confirm_stats_reset();
                gdb_println!("riscv confirm : counters cleared");
            }
            _ => gdb_println!("usage: mon riscv_confirm [reset]"),
        }
    }
    let s = bmp::bmp_riscv_confirm_stats();
    gdb_println!(
        "riscv confirm : windows=",
        s[0],
        " disagreeing=",
        s[1],
        " elements=",
        s[2]
    );
    gdb_println!(
        "riscv confirm : faults=",
        s[3],
        " latched_cmderr=",
        s[4],
        " autoexec_retries=",
        s[5]
    );
    gdb_println!(
        "riscv confirm : last @ ",
        Hex(s[6]),
        " pass1=",
        Hex(s[7]),
        " pass2=",
        Hex(s[8])
    );
    encoder::reply_ok();
    true
}
//
pub(crate) fn print_help_tree(prefix: &str, helptree: &[CommandTree]) {
    let mut mxsize: usize = 0;
    for i in helptree {
        if !i.help.is_empty() && i.command.len() > mxsize {
            mxsize = i.command.len();
        }
    }
    if mxsize > MAX_SPACE {
        panic!("padding too big");
    }
    for i in helptree {
        if i.help.is_empty() {
            continue;
        }
        let cmd = i.command;
        let len = cmd.len();
        let pad = core::str::from_utf8(&spacebar[..(mxsize - len)]).unwrap();
        gdb_print!(prefix, &cmd, " ", &pad);
        gdb_println!(": ", &(i.help));
    }
}
//
fn _mon_help(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("Help, use mon [cmd] [params] :\n");
    gdb_print!("-----------------------------\n");
    print_help_tree("mon ", &mon_command_tree);
    if let Some(x) = get_custom_target_command() {
        gdb_print!("Board specific commands :\n");
        gdb_print!("------------------------\n");
        print_help_tree("mon ", x);
    }
    // Which driver owns the block above (it is only shown when that driver
    // declared commands and its target is the one attached).
    let driver_name = bmp::bmp_get_target_name();
    if !driver_name.is_empty() {
        gdb_println!("Attached driver : ", driver_name);
    }

    encoder::reply_ok();
    true
}
//
#[cfg(feature = "hosted")]
fn _mon_test(_command: &str, _args: &[&str]) -> bool {
    true
}
#[cfg(not(feature = "hosted"))]
fn _mon_test(_command: &str, _args: &[&str]) -> bool {
    //unsafe {
    //resetTest();
    //}
    //let pin: rnPin = rnPin::PB6;
    //let mut state: bool = false;
    //gdb_print!("starting test\n");
    //pinMode(pin, lnOUTPUT);
    //loop {
    //digital_write(pin, state);
    //state = !state;
    //delay_ms(2000);
    //}
    //encoder::reply_ok();
    true
}
fn _mon_test2(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}

//
fn _boards(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("Support enabled for :\n");
    let boards = bmp::bmp_supported_boards();
    for i in boards.split(':') {
        gdb_println!("\t", i);
    }
    encoder::reply_ok();
    true
}

//
pub fn _voltage(_command: &str, _args: &[&str]) -> bool {
    let voltage = bmp::bmp_get_target_voltage();
    let voltage32: u32 = (voltage * 1000.0f32) as u32;

    gdb_println!("Voltage (mv) : ", voltage32);
    encoder::reply_ok();
    true
}

//
// Execute command
//
const MAX_RCMD_SIZE: usize = 128;
/// Handle `qRcmd` — execute a monitor command (hex-encoded).
///
/// Decodes the hex-encoded command string, splits it into command + args,
/// and dispatches to the appropriate handler (target-specific first, then
/// generic).
pub fn _qRcmd(_command: &str, args: &[&str]) -> bool {
    if args.len() != 1 {
        gdb_print!("qRCmd : wrong args\n");
        return false;
    }
    if args[0].len() > 2 * MAX_RCMD_SIZE {
        //
        gdb_println!("qRCmd : too long ", args[0].len());
        return false;
    }
    // The command is hex encoded, decode it
    let mut out: [u8; MAX_RCMD_SIZE] = [0; MAX_RCMD_SIZE];
    let rcmd = u8_hex_string_to_u8s(args[0].as_bytes(), &mut out);

    // Check if it is in the per target tree ...
    if let Some(x) = get_custom_target_command()
        && exec_one(x, rcmd, false)
    {
        return true;
    }
    // if not, look the generic one
    exec_one(&mon_command_tree, rcmd, false)
}

/// Handle `mon version` — display firmware version.
pub fn _get_version(_command: &str, _args: &[&str]) -> bool {
    gdb_println!("", bmp::bmp_get_version());
    encoder::reply_ok();
    true
}

pub fn swindle_display_targets() -> bool {
    crate::gdb_print!("Available Targets:\n");
    crate::gdb_print!("No. Att Driver\n");
    let mut count = 0usize;
    let mut cur = unsafe { crate::target::target_list };
    while !cur.is_null() {
        count += 1;
        let view = crate::target::TargetView::new(cur);
        let attached = if view.attached() { "*" } else { " " };
        let driver = view.driver_str().unwrap_or("");
        let core = view.core_str().unwrap_or("");
        if driver == "ARM Cortex-M" {
            crate::gdb_print!(
                "***",
                count,
                " ",
                attached,
                " Unknown ",
                driver,
                " Designer 0x",
                Hex(view.designer_code()),
                " Part ID 0x",
                Hex(view.part_id()),
                " ",
                core,
                "\n"
            );
        } else {
            crate::gdb_print!(" ", count, "   ", attached, "  ", driver, " ", core, "\n");
        }
        cur = unsafe { (*cur).next };
    }
    if count == 0 {
        crate::gdb_print!("No usable targets found.\n");
        false
    } else {
        true
    }
}

/// Handle `mon vector_catch` — catch exception vectors.
pub fn _vector_catch(_command: &str, args: &[&str]) -> bool {
    let target = crate::target::get_cur_target();
    if target.is_null() || !crate::arm::cortexm::is_cortexm(target) {
        crate::gdb_println!("vector_catch: no Cortex-M target attached");
        encoder::reply_ok();
        return false;
    }
    let res = crate::arm::cortexm::vector_catch_cmd(target, args);
    encoder::reply_ok();
    res
}

pub fn common_scan<F>(name: &str, scan_fn: F) -> bool
where
    F: FnOnce() -> bool,
{
    bmplog!("{}:\n", name);
    // The scan may detach from the current target and attach a different one:
    // cached lines are target-specific and must be dropped.
    crate::mem_cache::invalidate();

    if !scan_fn() {
        crate::gdb_println!("{} failed!", name);
        bmpwarning!("{} failed!\n", name);
        return false;
    }
    swindle_display_targets();
    os_detach();
    encoder::reply_ok();
    true
}

/// Handle `mon targets` — display discovered targets.
pub fn _targets(_command: &str, _args: &[&str]) -> bool {
    swindle_display_targets();
    encoder::reply_ok();
    true
}

/*
   Detect stuff connected to the SWD interface
   Try to use the fastest speed
*/
/// Handle `mon swdp_scan` — probe SWD bus for ARM devices.
pub fn _swdp_scan(_command: &str, _args: &[&str]) -> bool {
    common_scan("swdp_scan", bmp::swdp_scan)
}
/*
   Detect stuff connected to the SWD interface
   Try to use the fastest speed
*/
/// Handle `mon rvswdp_scan` — probe for RISC-V devices.
pub fn _rvswdp_scan(_command: &str, _args: &[&str]) -> bool {
    common_scan("rvswdp_scan", bmp::rvswdp_scan)
}
// NOTE: the SDI handlers that used to sit here (_sdi_scan, _sdi_wire) moved to
// the optional SDI module crate::riscv_extra — riscv_extra_sdi.rs with the
// `sdi` cargo feature (CMake: SWINDLE_WITH_SDI), riscv_extra_sdi_stubs.rs
// without it. The two tree entries above are the only thing this file knows
// about SDI, and they resolve either way; the C half of the seam is
// swindle/include/bmp_riscv_extra.h.
/*
 *
 */
/// Handle `mon ws` — get/set SWD wait states.
pub fn _ws(_command: &str, args: &[&str]) -> bool {
    let len = args.len();
    if len > 0 {
        // ok we have an input
        let ws = ascii_string_decimal_to_u32(args[0]);
        bmp::bmp_set_wait_state(ws);
    }
    let w: u32 = bmp::bmp_get_wait_state();
    gdb_println!("wait states are now ", w);
    encoder::reply_ok();
    true
}
// convert_param_to_integer() now lives in parsing_util: it is a plain parsing
// helper, and the SDI handler in crate::riscv_extra uses it too.
/*
 *
 *
 */
/// Handle `mon fq` / `mon frequency` — get/set SWD clock frequency.
#[unsafe(no_mangle)]
pub fn _fq(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        let f: u32 = bmp::bmp_get_frequency();
        gdb_println!("current frequency is ", f);
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
    gdb_println!("frequency is now ", w);
    encoder::reply_ok();
    true
}
/**
 * Phase 0 instrumentation: `mon memlog [on|off|reset]`.
 * - no arg       -> print current counters and note the log state
 * - `on` / `off` -> enable/disable per-call [MEMLOG] lines
 * - `reset`      -> zero the SWD/RVSWD wire-transaction counters
 */
fn _memlog(_command: &str, args: &[&str]) -> bool {
    let (swd, rv) = bmp::bmp_mem_counts();
    if args.is_empty() {
        gdb_println!(
            "MEMLOG: swd_tx=",
            swd,
            " rv_tx=",
            rv,
            " (use 'mon memlog on' to enable per-call logging)"
        );
    } else {
        match args[0] {
            "on" => {
                bmp::bmp_set_mem_log(true);
                gdb_println!("MEMLOG on (swd_tx=", swd, " rv_tx=", rv, ")");
            }
            "off" => {
                bmp::bmp_set_mem_log(false);
                gdb_println!("MEMLOG off (swd_tx=", swd, " rv_tx=", rv, ")");
            }
            "reset" => {
                bmp::bmp_mem_counts_reset();
                gdb_println!("MEMLOG counters reset");
            }
            _ => gdb_println!("unknown memlog arg: ", args[0]),
        }
    }
    encoder::reply_ok();
    true
}
#[unsafe(no_mangle)]
static autoreset: AtomicU32 = AtomicU32::new(1);
pub fn set_enable_reset(nw: u32) {
    autoreset.store(nw, Ordering::Relaxed);
}
pub fn get_enable_reset() -> u32 {
    autoreset.load(Ordering::Relaxed)
}
pub fn _enable_reset_pin(_command: &str, args: &[&str]) -> bool {
    let ret: bool = string_to_bool(args[0]);
    set_enable_reset(ret as u32);
    encoder::reply_ok();
    gdb_println!("enable reset pin is now ", get_enable_reset());
    true
}
/*
 *
 *
 *
 */
/// Register the `mon` commands a Rust target driver declares.
///
/// Called by the driver dispatchers (`riscv/driver_registry.rs`, `arm/driver_registry.rs`) at
/// probe time with `commands()` / `command_help()`; the entry is stored under the driver name,
/// so any number of drivers can declare their own commands without `mon.rs` knowing about them.
pub fn register_driver_commands(driver_name: &'static str, commands: &'static [CommandTree]) {
    // A driver `name()` doubles as a C string (`"CH32V303\0"`), while the
    // name handed back by `bmp_get_target_name()` stops at the terminator: compare
    // the C-visible name on both sides.
    let driver_name = driver_name.trim_end_matches('\0');
    let mut sets = driverCommandSets.get();
    let mut free_slot = None;
    for (index, slot) in sets.iter_mut().enumerate() {
        match slot {
            // Same family: the ARM/RISC-V drivers expose one command set per crate, which
            // they re-register under the concrete part name on every probe. Reuse the slot
            // (comparing the command table identity) instead of burning one per part.
            Some((name, cmds)) if *name == driver_name || core::ptr::eq(*cmds, commands) => {
                *slot = Some((driver_name, commands));
                driverCommandSets.set(sets);
                return;
            }
            None if free_slot.is_none() => free_slot = Some(index),
            _ => (),
        }
    }
    match free_slot {
        Some(index) => {
            sets[index] = Some((driver_name, commands));
            driverCommandSets.set(sets);
        }
        None => gdb_println!("mon : driver command table full, ", driver_name, " ignored"),
    }
}

/// Install the monitor commands of the attached target's driver.
///
/// Looks the driver name up in the sets registered by `register_driver_commands()`
/// and installs that command tree; no match clears them, so a driver that
/// declares nothing (or a C-driven target) exposes only the generic `mon` tree.
pub fn add_target_commands(target_name: &str) {
    // `bmp_get_target_name()` is the C `target_s::driver`, i.e. a NUL-terminated name.
    let target_name = target_name.trim_end_matches('\0');
    for set in driverCommandSets.get().iter().flatten() {
        if !set.0.is_empty() && target_name.starts_with(set.0) {
            set_custom_target_command(set.1);
            return;
        }
    }
    clear_custom_target_command();
}

/*
 *
 *
 */
/// Clear any registered target-specific commands.
pub fn clear_custom_target_command() {
    targetCommandTree.set(None);
}
/*
 *
 *
 */
/// Register a custom command tree for the current target.
pub fn set_custom_target_command(commandTree: &'static [CommandTree]) {
    targetCommandTree.set(Some(commandTree));
}
fn _map(_command: &str, _args: &[&str]) -> bool {
    let ram = bmp::bmp_get_mapping(bmp::mapping::Ram);
    for i in ram {
        gdb_print!(" RAM   : start=", Hex(i.start_address));
        gdb_print!(" size=", i.length / 1024, " kB\n");
    }
    let flash = bmp::bmp_get_mapping(bmp::mapping::Flash);
    for i in flash {
        gdb_print!(" FLASH   : start=", Hex(i.start_address));
        gdb_print!(" size=", i.length / 1024, " kB\n");
    }
    encoder::reply_ok();
    true
}
/*
 * `mon xml reg|registers|target`  : the target.xml GDB receives (registers)
 * `mon xml mem|memory|map|memory-map` : the memory-map XML GDB receives
 */
/// Print the `mon xml` usage lines.
fn _xml_usage() {
    gdb_print!("usage: mon xml reg    -> target.xml : the register description GDB receives\n");
    gdb_print!("       mon xml memory -> memory-map XML : the memory map GDB receives\n");
}

/// Handle `mon xml <reg|memory>` — dump the XML description the probe answers GDB with.
///
/// `reg` prints `target.xml` (the registers and their features), `memory` prints the
/// `<memory-map>` document. Both are generated by the same code that serves
/// `qXfer:features:read` / `qXfer:memory-map:read`, so what is printed is exactly what
/// GDB is told, one tag per line.
fn _xml(_command: &str, args: &[&str]) -> bool {
    match args.first().copied() {
        Some("reg") | Some("registers") | Some("target") => {
            crate::target_xml::dump_target_xml();
            encoder::reply_ok();
            true
        }
        Some("mem") | Some("memory") | Some("map") | Some("memory-map") => {
            crate::memory_map_xml::dump_memory_map_xml();
            encoder::reply_ok();
            true
        }
        _ => {
            _xml_usage();
            encoder::reply_e01();
            false
        }
    }
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
/// Get the currently registered target-specific command tree.
pub fn get_custom_target_command() -> Option<&'static [CommandTree]> {
    targetCommandTree.get()
}
/*
 *
 *
 */
/// Handle `mon delay` — wait N milliseconds.
/// Handle `mon stub_stats [reset]` — what the flash-stub runner and the drivers actually did.
///
/// A load can report success through paths that are invisible in its timing: the stub may
/// never have reported its halt (500 ms of polling per block), or the block's content check
/// may have failed and the driver re-programmed it through the register path. Those are
/// counted here, per session, so a test can read one load's worth (`reset` first) instead of
/// inferring them.
pub fn _stub_stats(_command: &str, args: &[&str]) -> bool {
    if let Some(arg) = args.first() {
        if *arg == "reset" {
            crate::riscv::stub::stub_stats_reset();
            gdb_println!("stub stats : counters cleared");
        } else {
            gdb_println!("usage: mon stub_stats [reset]");
        }
    }
    let s = crate::riscv::stub::stub_stats();
    gdb_println!(
        "stub stats : runs=",
        s[0],
        " halted=",
        s[1],
        " repairs=",
        s[2]
    );
    gdb_println!(
        "stub stats : wait_ms=",
        s[3],
        " hold_ms=",
        s[4]
    );
    if s[0] != s[1] {
        gdb_println!(
            "stub stats : last unruly run pc=0x",
            Hex(s[5]),
            " sp=0x",
            Hex(s[6]),
            " code=0x",
            Hex(s[7]),
            ",0x",
            Hex(s[8])
        );
    }
    encoder::reply_ok();
    true
}

/// Handle `mon stub_hold [<ms>|auto]` — the flash-stub silence, per hardware page.
///
/// The hold is the period a flash driver stays off the debug module after resuming its
/// programming stub, because a CH32V00x corrupts its own flash programming if the debug
/// module is accessed while the stub runs (doc/ch32v00x.md). Its value is a property of
/// the part, so it is swept on real pages: `mon stub_hold 0`, load a block, check the
/// content, repeat; `mon stub_hold auto` puts the driver's own number back.
pub fn _stub_hold(_command: &str, args: &[&str]) -> bool {
    if let Some(arg) = args.first() {
        if *arg == "auto" {
            crate::riscv::stub::set_stub_hold_override_ms(None);
        } else {
            let (ok, ms) = convert_param_to_integer(arg);
            if !ok {
                return false;
            }
            crate::riscv::stub::set_stub_hold_override_ms(Some(ms));
        }
    }
    match crate::riscv::stub::stub_hold_override_ms() {
        Some(ms) => gdb_println!("stub hold : ", ms, " ms per page (override)"),
        None => gdb_print!("stub hold : the part's own value (override off)\n"),
    }
    encoder::reply_ok();
    true
}

pub fn _delay(_command: &str, args: &[&str]) -> bool {
    let ret: bool;
    let val: u32;
    (ret, val) = convert_param_to_integer(args[0]);
    if !ret {
        return false;
    }
    delay_ms(val);
    encoder::reply_ok();
    true
}
/*
 *
 *
 */
/// Handle `mon set_reset_pin` — manually set reset pin state.
#[unsafe(no_mangle)]
pub fn _set_reset_pin(_command: &str, args: &[&str]) -> bool {
    let ret: bool = string_to_bool(args[0]);
    // Asserting NRST may reset the target: drop cached lines to be safe.
    crate::mem_cache::invalidate();
    crate::reset_pin::ResetPin::set_internal(ret);
    encoder::reply_ok();
    gdb_println!("reset pin is now ", ret);
    true
}

fn set_nrst(set: bool) {
    crate::reset_pin::ResetPin::set_internal(set);
}
/// Set the NRST pin value with delay handling (C-callable).
#[unsafe(no_mangle)]
#[cfg(not(feature = "hosted"))]
pub fn platform_nrst_set_val(set: u32) {
    if get_enable_reset() == 0 {
        return;
    }
    swindle_nrst_set_val(set);
}
/// Set the NRST pin with configurable pulse/holdoff delays.
#[unsafe(no_mangle)]
pub fn swindle_nrst_set_val(set: u32) {
    let is_set = set != 0;
    set_nrst(is_set);
    let key = if is_set {
        RESET_PULSE_DURATION
    } else {
        RESET_HOLDOFF_DURATION
    };
    delay_ms(settings::get_or_default(key, 10));
}
/// Handle `mon breakpoint_info` — show HW breakpoint/watchpoint count.
pub fn _breakpoint_count(_command: &str, _args: &[&str]) -> bool {
    let (bkpt, wtch) = bmp::bmp_watchpoint_breakpoint_count();
    // A Rust-driven RISC-V target has no C-side trigger bookkeeping: its count comes
    // from the driver that probed the hart's trigger unit.
    let bkpt = if bkpt == 0 {
        crate::riscv::driver_registry::active_hw_breakpoint_count()
    } else {
        bkpt
    };
    gdb_println!("\t HW Breakpoints    : ", bkpt);
    gdb_println!("\t HW Watchpoints    : ", wtch);
    encoder::reply_ok();
    true
}

fn parse_csr_name_or_num(arg: &str) -> Option<u32> {
    match arg {
        "mstatus" => Some(RV_CSR_MSTATUS as u32),
        "misa" => Some(RV_CSR_MISA as u32),
        "mie" => Some(RV_CSR_MIE as u32),
        "mtvec" => Some(RV_CSR_MTVEC as u32),
        "mscratch" => Some(RV_CSR_MSCRATCH as u32),
        "mepc" => Some(RV_CSR_MEPC as u32),
        "mcause" => Some(RV_CSR_MCAUSE as u32),
        "mtval" => Some(RV_CSR_MTVAL as u32),
        "mip" => Some(RV_CSR_MIP as u32),
        "tselect" => Some(RV_CSR_TSELECT as u32),
        "tdata1" => Some(RV_CSR_TDATA1 as u32),
        "tdata2" => Some(RV_CSR_TDATA2 as u32),
        "tdata3" => Some(RV_CSR_TDATA3 as u32),
        "tinfo" => Some(RV_CSR_TINFO as u32),
        "dcsr" => Some(RV_CSR_DCSR as u32),
        "dpc" => Some(RV_CSR_DPC as u32),
        "dscratch0" => Some(RV_CSR_DSCRATCH0 as u32),
        "dscratch1" => Some(RV_CSR_DSCRATCH1 as u32),
        _ => {
            let s = arg
                .strip_prefix("0x")
                .or_else(|| arg.strip_prefix("0X"))
                .unwrap_or(arg);
            let mut val: u32 = 0;
            if s.is_empty() {
                return None;
            }
            for &b in s.as_bytes() {
                let digit = match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return None,
                };
                val = (val << 4) | digit;
            }
            Some(val)
        }
    }
}

/// Handle `mon csr <name|hex> [value]` — read or write RISC-V CSR.
pub fn _csr(_command: &str, args: &[&str]) -> bool {
    if !bmp::bmp_is_riscv() {
        gdb_println!("Error: Target architecture is not RISC-V.");
        encoder::reply_e01();
        return false;
    }

    if args.is_empty() {
        gdb_println!("usage: mon csr <name|number> [value]");
        encoder::reply_e01();
        return false;
    }

    let csr_arg = args[0].trim();
    let csr_num = match parse_csr_name_or_num(csr_arg) {
        Some(num) => num,
        None => {
            gdb_println!("Invalid CSR name or number: ", csr_arg);
            encoder::reply_e01();
            return false;
        }
    };

    if args.len() > 1 {
        let val_arg = args[1].trim();
        let val = match parse_csr_name_or_num(val_arg) {
            Some(v) => v,
            None => {
                gdb_println!("Invalid CSR value: ", val_arg);
                encoder::reply_e01();
                return false;
            }
        };
        if bmp::bmp_riscv_write_csr(csr_num, val) {
            gdb_println!("CSR 0x", Hex(csr_num), " written with ", Hex(val));
            encoder::reply_ok();
            true
        } else {
            gdb_println!("Failed to write CSR 0x", Hex(csr_num));
            encoder::reply_e01();
            false
        }
    } else {
        match bmp::bmp_riscv_read_csr(csr_num) {
            Some(val) => {
                gdb_println!("CSR 0x", Hex(csr_num), " = ", Hex(val));
                encoder::reply_ok();
                true
            }
            None => {
                gdb_println!("Failed to read CSR 0x", Hex(csr_num));
                encoder::reply_e01();
                false
            }
        }
    }
}

//-- EOF --
