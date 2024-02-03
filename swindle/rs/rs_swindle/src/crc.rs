/*
 *
 */
crate::setup_log!(false);
use crate::{bmplog, bmpwarning};
use crate::util::xmin;

// qCRC:addr hex,length hex’
// return Ccrc32 hex
const GDB_CRC_ALG: crc::Algorithm<u32> = crc::Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: 0xffffffff,
    refin: false,
    refout: false,
    xorout: 0x0000,
    check: 0xaee7,
    residue: 0x0000,
};
const CRC_BUFFER_SIZE: usize = 128;
const crc32_cksum: crc::Crc<u32> = crc::Crc::<u32>::new(&GDB_CRC_ALG);
/*
 */
pub fn do_local_crc32(address: u32, length: u32) -> (bool, u32) {   
    let mut digest = crc32_cksum.digest();
    let mut buffer: [u8; CRC_BUFFER_SIZE] = [0; CRC_BUFFER_SIZE]; // should be big enough!

    bmpwarning!("CRC : Adr 0x{:x}  ", address);
    bmpwarning!("len {}\n", length);
    //
    let tail = address + length;
    let mut adr: u32 = address;
    while adr < tail {
        let rd: usize = xmin(tail - adr, CRC_BUFFER_SIZE as u32) as usize;
        if !crate::bmp::bmp_read_mem(adr, &mut buffer[0..rd]) {
            return (false, 0);
        }
        digest.update(&buffer[0..rd]);
        adr += rd as u32;
    }
    let crc = digest.finalize();
    bmpwarning!("XXCRC={:x}\n", crc);
    (true, crc)
}
//----------
// Local
// In that case we compute the CRC directly on the BMP itself
//----------
//#[cfg(target_os = "none")]
pub fn abstract_crc32(address: u32, len: u32, crc: &mut u32) -> bool {
    let status: bool;
    bmpwarning!("local CRC\n");
    (status, *crc) = do_local_crc32(address, len); // Native
    status
}
//----------
// Remote
// in that case we do a RPC call so that the remote BMP does the computation
// should be much faster
//----------
#[cfg(not(target_os = "none"))]
use crate::hosted_rpc::remote_rpc::remote_crc32;

#[cfg(not(target_os = "none"))]
pub fn abstract_crc32_not(address: u32, len: u32, crc: &mut u32) -> bool {
    bmpwarning!("remote CRC\n");
    remote_crc32(address, len, crc) // remote
}
// EOF
