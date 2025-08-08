use crate::bmp;
use crate::commands::mon::get_enable_reset;
use crate::encoder::encoder;
use numtoa::NumToA;

static mut running: bool = false;

crate::setup_log!(false);
crate::gdb_print_init!();

use crate::gdb_print;
#[derive(PartialEq, Eq)]
pub enum HaltState {
    Running,
    Error,
    Request,
    Stepping,
    Breakpoint,
    Watchpoint(u32),
    Fault,
}
fn set_running(r: bool) {
    unsafe {
        running = r;
    }
}
pub fn target_halt() {
    set_running(false);
    bmp::bmp_target_halt();
    reply_2("T", 2);
}

pub fn target_is_running() -> bool {
    unsafe { running }
}

//
fn reply_2(prefix: &str, num: u32) {
    let mut buffer: [u8; 20] = [0; 20];
    let mut e = encoder::new();
    e.begin();
    e.add(prefix);
    if num < 16 {
        e.add("0");
    }
    e.add(num.numtoa_str(16, &mut buffer));
    e.end();
}
//
fn reply_wp(prefix: &str, num: u32, prefix2: &str, num2: u32) {
    let mut buffer: [u8; 20] = [0; 20];
    let mut e = encoder::new();
    e.begin();
    e.add(prefix);
    if num < 16 {
        e.add("0");
    }
    e.add(num.numtoa_str(16, &mut buffer));
    e.add(prefix2);
    if num2 < 16 {
        e.add("0");
    }
    e.add(num2.numtoa_str(16, &mut buffer));
    e.add(";");
    e.end();
}

#[unsafe(no_mangle)]
extern "C" fn rngdbstub_poll() {
    // this is called regularily
    let check: bool;
    check = bmp::bmp_attached() && target_is_running();
    if check {
        match bmp::bmp_poll() {
            HaltState::Running => return,           // nothing to do !
            HaltState::Error => reply_2("X", 29),   // SIGLOST
            HaltState::Stepping => reply_2("T", 5), // SIGTRAP
            HaltState::Request => reply_2("T", 2),  // SIGINT
            HaltState::Watchpoint(wp) => reply_wp("T", 5, "watch:", wp), // SIGTRAP
            HaltState::Fault => reply_2("T", 11),   // SIGSEGV
            HaltState::Breakpoint => reply_2("T", 5), // SIGTRAP
                                                     //          _ => panic!("unsupported halt state"),
        }
        set_running(false);
    }
}

//
pub fn _R(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_bool(bmp::bmp_reset_target());
    true
}
pub fn _k(_command: &str, _args: &[&str]) -> bool {
    if get_enable_reset() != 0 {
        encoder::reply_bool(bmp::bmp_reset_target());
    } else {
        gdb_print!("reset disabled by mon enablereset\n");
        encoder::reply_ok();
    }
    true
}
//vCont[;action[:thread-id]]…’
//
pub fn _c(_command: &str, args: &[&str]) -> bool {
    _vCont("vCont", args)
}
pub fn _s(_command: &str, args: &[&str]) -> bool {
    _vCont("vCont;s", args)
}

pub fn _vCont(command: &str, _args: &[&str]) -> bool {
    if command.starts_with("vCont?") {
        let mut e = encoder::new();
        e.begin();
        e.add("vCont;c;s;t");
        e.end();
        return true;
    }
    if command.len() < 7
    // naked vcond
    {
        set_running(true);
        bmp::bmp_halt_resume(false);
        //encoder::reply_ok();
        return true;
    }
    let command_bytes = command.as_bytes();
    match command_bytes[6] {
        b'c' => {
            set_running(true);
            bmp::bmp_halt_resume(false);
            true
        }
        b's' => {
            set_running(true);
            bmp::bmp_halt_resume(true);
            true
        }
        b't' => {
            encoder::reply_e01(); // !! TODO !!
            true
        }
        _ => false,
    }
}
