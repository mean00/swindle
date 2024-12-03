/*
 *
 */
crate::setup_log!(false);
use crate::bmplog;
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

    bmplog!("CRC : Adr 0x{:x}  ", address);
    bmplog!("len {}\n", length);
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
    bmplog!("XXCRC={:x}\n", crc);
    (true, crc)
}
/*
TODO this does not work because we are not attached on the remote BMP
TODO
 */
//----------
// Local
// In that case we compute the CRC directly on the BMP itself
//----------
pub fn abstract_crc32(address: u32, len: u32, crc: &mut u32) -> bool {
    let mut status: bool;
    // can we use an optimized version ?
    (status, *crc) = crate::bmp::bmp_custom_crc32(address, len);
    bmplog!(" CRC custom 0x{:x}\n", *crc);
    if !status {
        (status, *crc) = do_local_crc32(address, len);
        bmplog!(" CRC sw 0x{:x}\n", *crc);
    }
    status
}
// EOF
