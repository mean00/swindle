use alloc::alloc::alloc;
use alloc::alloc::Layout;
use cty::size_t;

/**
 *
 */
#[cfg(feature = "hosted")]
extern crate std;
#[cfg(feature = "hosted")]
use std::print;


pub fn xswap(a: &mut isize, b: &mut isize) {
    core::mem::swap(&mut (*a), &mut (*b));
}
pub fn xmin<T: core::cmp::PartialOrd>(a: T, b: T) -> T {
    if a < b {
        return a;
    }
    b
}
pub fn xmax(a: isize, b: isize) -> isize {
    if b < a {
        return a;
    }
    b
}

pub fn xminu(a: usize, b: usize) -> usize {
    if a < b {
        return a;
    }
    b
}
pub fn xmaxu(a: usize, b: usize) -> usize {
    if b < a {
        return a;
    }
    b
}
//
//https://stackoverflow.com/questions/59232877/how-to-allocate-structs-on-the-heap-without-taking-up-space-on-the-stack-in-stab

//-----------
extern "C" {
    pub fn malloc(size: size_t) -> *mut cty::c_void;
}

//-----------
pub fn unsafe_slice_alloc<T>(count: usize) -> &'static mut [T] {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        core::slice::from_raw_parts_mut(ptr as *mut T, count)
    }
}

pub fn unsafe_array_alloc<T>(count: usize) -> *mut T {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        ptr as *mut T
    }
}

pub fn unsafe_box_allocate<T>() -> *mut T {
    let layout = Layout::new::<T>();
    unsafe { alloc(layout) as *mut T }
}

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
const CRC_BUFFER_SIZE: usize = 64;
const crc32_cksum: crc::Crc<u32> = crc::Crc::<u32>::new(&GDB_CRC_ALG);
/*
 */
pub fn do_crc32(address: u32, length : u32 ) -> (bool, u32) {
    // loop in crc
    
    let mut digest = crc32_cksum.digest();
    let mut buffer: [u8; CRC_BUFFER_SIZE] = [0; CRC_BUFFER_SIZE]; // should be big enough!

    //bmplog!("CRC : Adr 0x{:x}", address);
    //bmplog!("len {}\n", length);
    //
    //let crc = bmp_crc32(address,length);
    // preamble if any
    let tail = address + length;
    let mut adr: u32 = address;
    let mut block = 0;
    while adr < tail {
        block += 1;
        if block > 64
        // every 2k bytes or so
        {
            block = 0;
            //encoder::raw_send("+");
            //encoder::flush();
            //            encoder::raw_send_u8(&[0]);
            //            encoder::flush();
        }
        let rd: u32 = xmin(tail - adr, buffer.len() as u32);
        if !crate::bmp::bmp_read_mem(adr, &mut buffer[0..(rd as usize)]) {
            //bmpwarning!("CRC : cant read memory 0x{:x}\n", adr);
           // encoder::error(1);
            return (false,0);
        }

        digest.update(&buffer[0..(rd as usize)]);
        adr += rd;
    }
    let crc = digest.finalize();
    return (true,crc);
}