
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_mem_write};

crate::setup_log!(false);


// memory read m80070f6,4
pub fn _m(command : &str, _args : &Vec<&str>) -> bool
{
    if !crate::bmp::bmp_attached()
    {
        encoder::reply_e01(); 
        return true;
    }
    
    match crate::parsing_util::take_adress_length(&command[1..])
    {
        None => encoder::reply_e01(),
        Some( (adr,len) ) => 
            {
                
                let mut tmp  : [u8;16]= [0;16];
                let mut char_buffer : [u8;32]  =[ 0; 32];

                let mut current_address : u32 = adr;
                let mut left : usize = len as usize;

                let mut e  = encoder::new();
                e.begin();

                while left!=0
                {
                    let chunk : usize  = core::cmp::min(16,left) as usize;
                    crate::bmp::bmp_read_mem(current_address,&mut tmp[0..chunk]);
                    current_address += chunk as u32;
                    left -= chunk;
                    for i in 0..chunk
                    {
                        crate::parsing_util::u8_to_ascii_to_buffer( tmp[i]  , &mut char_buffer[(2*i)..]);
                    }
                    e.add_u8(&char_buffer[..(2*chunk)]);
                }

                e.end();
                
            }
    }
    return true;
  
}
/**
 *  \fn write memory
 * 
 */
pub fn _X(command : &str, args : &[u8]) -> bool
{    
    match crate::parsing_util::take_adress_length(&command[1..])
    {
        None                => encoder::reply_e01(),
        Some( (addr,len) )   => 
                            {
                                    bmplog1("adr",addr);
                                    bmplog1("len",len);
                                    let mut actual_len: usize = len as usize;
                                    if args.len()  > actual_len
                                    {
                                        actual_len=args.len() ;
                                    }
                                    if bmp_mem_write( addr,&args[0..actual_len])
                                    {
                                        encoder::reply_ok();
                                    }else{
                                        encoder::reply_e01();
                                    }
                            },
    };
    true
}

// EOF
