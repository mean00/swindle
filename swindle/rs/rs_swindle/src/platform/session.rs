//! Platform I/O session bracket hooks.

unsafe extern "C" {
    fn bmp_io_begin_session();
    fn bmp_io_end_session();
}

/// Signal the platform hardware that a target debug session is starting.
#[inline]
pub fn begin_session() {
    unsafe { bmp_io_begin_session(); }
}

/// Signal the platform hardware that a target debug session has ended.
#[inline]
pub fn end_session() {
    unsafe { bmp_io_end_session(); }
}
