
use rnarduino::rn_os_helper::log;

pub fn print_str(s: &str) {
    log(s);
    //unsafe {
      //  libc::write(1, s.as_ptr() as _, s.len());
      //  libc::write(1, "\n".as_ptr() as _, 1);
    //}
}
