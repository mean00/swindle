unsafe extern "C" {
    fn rngdb_send_data_c(sz: u32, ptr: *const cty::c_uchar);
    fn rngdb_output_flush_c();
}

pub fn rngdb_output_flush() {
    unsafe { rngdb_output_flush_c() }
}
pub fn rngdb_send_data(data: &str) {
    unsafe { rngdb_send_data_c(data.len() as u32, data.as_ptr()) }
}
pub fn rngdb_send_data_u8(data: &[u8]) {
    unsafe { rngdb_send_data_c(data.len() as u32, data.as_ptr()) }
}